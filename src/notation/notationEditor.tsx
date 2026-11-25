/**
 * The notation editor: a `node-type-editor` implementation for Aura notation records.
 *
 * Lab hosts the panel and keeps the record's value and undo history. Everything audio-specific
 * lives here: the piano roll, its playhead and transport, note audition and recording takes.
 * The services it talks to come from the component's params (`serviceId`).
 */

import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
  type ReactNode,
} from "react";
import type { EditorHost, EditorPanelProps, NodeTypeEditorImplementation } from "imp-lab-ui";
import { createAuditionController } from "./audition";
import {
  emptyNotation,
  notationFromUnknown,
  notationsEqual,
  secondsToBeats,
  summarizeNotation,
  type Notation,
} from "./notation";
import { PianoRollPanel } from "./PianoRollPanel";
import { createPrefsStore, type PianoRollUiEntry, type PrefsStore } from "./prefs";
import { applyTake, createRecordingController, takeFromTopic } from "./recording";

/** Commands the notation editor handles while its document is active. */
export const CMD_TOOL_DRAW = "notation.tool.draw";
export const CMD_TOOL_SELECT = "notation.tool.select";
export const CMD_TOOL_ERASE = "notation.tool.erase";
export const CMD_SNAP_TOGGLE = "notation.snap.toggle";
export const CMD_RECORD_TOGGLE = "notation.record.toggle";

/** Transport commands of the Aura pack's playback service. */
export const CMD_PLAY_START = "play.start";
export const CMD_PLAY_STOP = "play.stop";
export const CMD_PLAY_TOGGLE = "play.toggle";

const PLAYHEAD_POLL_MS = 33;

const prefsStores = new Map<string, PrefsStore>();

/** Prefs are per editor component, shared by all its open documents. */
function usePrefs(host: EditorHost) {
  let store = prefsStores.get(host.componentId);
  if (!store) {
    const h = host;
    store = createPrefsStore(
      () => h.loadPrefs<PianoRollUiEntry>(),
      (entry) => h.savePrefs(entry),
    );
    prefsStores.set(host.componentId, store);
  }
  const prefs = useSyncExternalStore(store.subscribe, store.get);
  return [prefs, store.set] as const;
}

/** Notation JSON for the playback service, starting from the parked playhead. */
function transportJson(notation: Notation, startBeats: number): string {
  return JSON.stringify({ ...notation, labStartBeats: startBeats });
}

export function NotationPanel({ value, onChange, host }: EditorPanelProps<Notation>) {
  const hostRef = useRef(host);
  hostRef.current = host;
  const valueRef = useRef(value);
  valueRef.current = value;
  const serviceId = typeof host.params.serviceId === "string" ? host.params.serviceId : null;

  const [prefs, setPrefs] = usePrefs(host);
  const prefsRef = useRef(prefs);
  prefsRef.current = prefs;
  const [playhead, setPlayhead] = useState(0);
  const playheadRef = useRef(playhead);
  playheadRef.current = playhead;
  const [playing, setPlaying] = useState(false);
  const [livePlayhead, setLivePlayhead] = useState<number | null>(null);

  const audition = useMemo(
    () =>
      serviceId
        ? createAuditionController((event) => hostRef.current.sendServiceEvent(serviceId, event))
        : null,
    [serviceId],
  );
  // Leaving the tab releases anything still sounding.
  useEffect(() => {
    if (!host.active) {
      audition?.allOff();
    }
  }, [audition, host.active]);

  const recorder = useMemo(
    () =>
      serviceId
        ? createRecordingController((body) => hostRef.current.requestService(serviceId, body))
        : null,
    [serviceId],
  );
  const [captureTopic, setCaptureTopic] = useState<string | null>(null);
  useEffect(() => {
    let cancelled = false;
    void recorder?.info().then((topic) => {
      if (!cancelled) setCaptureTopic(topic);
    });
    return () => {
      cancelled = true;
    };
  }, [recorder]);
  const take = takeFromTopic(host.useTopic(captureTopic));
  const [recordingActive, setRecordingActive] = useState(false);
  const recordingRef = useRef(false);
  /** Whether the take topic has shown this take recording (to notice an external stop). */
  const takeSeenRef = useRef(false);

  const stopTransportState = () => {
    setPlaying(false);
    setLivePlayhead(null);
  };

  /** Stops the take and adds its notes to the notation as one edit. */
  const stopRecording = useCallback(async () => {
    if (!recordingRef.current || !recorder) {
      return;
    }
    recordingRef.current = false;
    setRecordingActive(false);
    stopTransportState();
    takeSeenRef.current = false;
    try {
      const notes = await recorder.stop();
      if (notes.length > 0) {
        onChange(applyTake(valueRef.current, notes));
        hostRef.current.setStatus(`Recorded ${notes.length} note${notes.length === 1 ? "" : "s"}`);
      } else {
        hostRef.current.setStatus("Recording stopped · no notes");
      }
    } catch (err) {
      hostRef.current.setStatus(`Recording failed · ${String(err)}`);
    }
  }, [onChange, recorder]);

  const toggleRecording = useCallback(async () => {
    if (recordingRef.current) {
      await stopRecording();
      return;
    }
    if (!recorder || !captureTopic) {
      hostRef.current.setStatus("Recording needs a capture service");
      return;
    }
    try {
      audition?.allOff();
      const click = prefsRef.current.click;
      await recorder.start(valueRef.current, playheadRef.current, click);
      recordingRef.current = true;
      takeSeenRef.current = false;
      setRecordingActive(true);
      setPlaying(true);
      hostRef.current.setStatus(click ? "Recording · count-in" : "Recording");
    } catch (err) {
      hostRef.current.setStatus(`Recording failed · ${String(err)}`);
    }
  }, [audition, captureTopic, recorder, stopRecording]);

  // The transport stopped some other way (menu Stop): collect the take.
  useEffect(() => {
    if (!recordingActive) {
      return;
    }
    if (take.recording) {
      takeSeenRef.current = true;
    } else if (takeSeenRef.current) {
      void stopRecording();
    }
  }, [recordingActive, stopRecording, take.recording]);

  // Leaving the notation (tab switch or close) ends the take there.
  useEffect(() => {
    if (!host.active && recordingRef.current) {
      void stopRecording();
    }
  }, [host.active, stopRecording]);
  useEffect(
    () => () => {
      if (recordingRef.current) void recorder?.stop();
    },
    [recorder],
  );

  /** Play commands carry this notation and the parked playhead. */
  const runTransport = useCallback(async (id: string) => {
    const h = hostRef.current;
    try {
      await h.runCommand(id, transportJson(valueRef.current, playheadRef.current));
      if (id === CMD_PLAY_STOP) {
        stopTransportState();
        h.setStatus("Playback stopped");
      } else if (id === CMD_PLAY_START) {
        setPlaying(true);
        h.setStatus("Playback started");
      } else {
        // Status is post-toggle: playing means it just started.
        const status = await h.playbackStatus().catch(() => ({ playing: true }));
        if (status.playing) setPlaying(true);
        else stopTransportState();
        h.setStatus("Playback toggled");
      }
    } catch (err) {
      h.setStatus(`Command failed · ${id}: ${String(err)}`);
    }
  }, []);

  const handleCommand = useCallback(
    (id: string): boolean => {
      const set = (patch: Partial<typeof prefs>, status: string) => {
        setPrefs({ ...prefsRef.current, ...patch });
        hostRef.current.setStatus(status);
      };
      switch (id) {
        case CMD_TOOL_DRAW:
          set({ tool: "draw" }, "Tool · Draw");
          return true;
        case CMD_TOOL_SELECT:
          set({ tool: "select" }, "Tool · Select");
          return true;
        case CMD_TOOL_ERASE:
          set({ tool: "erase" }, "Tool · Erase");
          return true;
        case CMD_SNAP_TOGGLE: {
          const snapEnabled = !prefsRef.current.snapEnabled;
          set({ snapEnabled }, snapEnabled ? "Snap on" : "Snap off");
          return true;
        }
        case CMD_RECORD_TOGGLE:
          void toggleRecording();
          return true;
        case CMD_PLAY_START:
        case CMD_PLAY_STOP:
        case CMD_PLAY_TOGGLE:
          // Space (or Play › Stop) during a take ends the take and keeps its notes.
          if (recordingRef.current && id !== CMD_PLAY_START) {
            void stopRecording();
            return true;
          }
          if (!hostRef.current.hasCommand(id)) {
            return false;
          }
          void runTransport(id);
          return true;
        default:
          return false;
      }
    },
    [runTransport, setPrefs, stopRecording, toggleRecording],
  );
  const handleCommandRef = useRef(handleCommand);
  handleCommandRef.current = handleCommand;
  useEffect(() => host.onCommand((id) => handleCommandRef.current(id)), [host]);

  // The playhead follows the playback clock; it snaps back when playback stops.
  useEffect(() => {
    if (!playing) {
      setLivePlayhead(null);
      return;
    }
    let cancelled = false;
    const handle = window.setInterval(() => {
      void hostRef.current
        .playbackStatus()
        .then((status) => {
          if (cancelled) return;
          if (!status.playing) {
            stopTransportState();
            return;
          }
          setLivePlayhead(secondsToBeats(status.positionSecs, valueRef.current.tempoBpm));
        })
        .catch(() => {
          // No playback service; ignore.
        });
    }, PLAYHEAD_POLL_MS);
    return () => {
      cancelled = true;
      window.clearInterval(handle);
    };
  }, [playing]);

  return (
    <PianoRollPanel
      notation={value}
      playheadBeats={playing && livePlayhead != null ? livePlayhead : playhead}
      prefs={prefs}
      audition={audition}
      recording={
        captureTopic
          ? {
              active: recordingActive,
              countIn: recordingActive && take.countIn,
              takeNotes: recordingActive ? take.notes : [],
              onToggle: () => void toggleRecording(),
            }
          : null
      }
      toolbarExtra={host.statusItems as ReactNode}
      onPrefsChange={setPrefs}
      onNotationChange={onChange}
      onPlayheadChange={setPlayhead}
    />
  );
}

/** `node-type-editor` implementation for notation records. */
export const notationEditor: NodeTypeEditorImplementation<Notation> = {
  kind: "node-type-editor",
  empty: () => emptyNotation(),
  parse: (stored) => notationFromUnknown(stored ?? emptyNotation()),
  summarize: summarizeNotation,
  properties: (notation) => [
    { label: "Events", value: String(notation.events.length) },
    { label: "Tempo", value: String(notation.tempoBpm) },
  ],
  equals: notationsEqual,
  Panel: NotationPanel,
};
