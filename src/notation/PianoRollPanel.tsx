/** Piano-roll editor for Aura notation records. */

import { useCallback, useEffect, useRef, useState, type ReactNode } from "react";
import {
  Button,
  Checkbox,
  Group,
  SegmentedControl,
  Select,
  Text,
} from "@mantine/core";
import type { AuditionController } from "./audition";
import { applyNotationEdit } from "./edits";
import {
  isBlackKey,
  snapBeats,
  snapResolutionBeats,
  semitoneLabel,
  SNAP_RESOLUTIONS,
  type Notation,
  type NotationNote,
  type NotationTool,
  type SnapResolution,
} from "./notation";
import classes from "./PianoRollPanel.module.css";

const PIANO_WIDTH = 56;
const RULER_HEIGHT = 20;
const PITCH_MIN = 36; // C2
const PITCH_MAX = 84; // C6
const KEY_HEIGHT = 14;
const BEAT_WIDTH = 48;
const EDGE_PX = 6;
const SCALE_MIN = 0.35;
const SCALE_MAX = 3;

export type PianoRollUiPrefs = {
  tool: NotationTool;
  snapEnabled: boolean;
  snapResolution: SnapResolution;
  /** Sound notes while they are placed, pressed, or dragged. */
  audition: boolean;
  /** Metronome click and one-bar count-in while recording. */
  click: boolean;
};

/** Live recording state; `null` when no service offers capture. */
export type PianoRollRecording = {
  active: boolean;
  countIn: boolean;
  /** The take so far, drawn over the notation. */
  takeNotes: NotationNote[];
  onToggle: () => void;
};

export type PianoRollPanelProps = {
  notation: Notation;
  playheadBeats: number;
  prefs: PianoRollUiPrefs;
  /** Live note audition; `null` when no playback service is loaded. */
  audition: AuditionController | null;
  recording: PianoRollRecording | null;
  /** Extra toolbar content (extension status items). */
  toolbarExtra?: ReactNode;
  onPrefsChange: (prefs: PianoRollUiPrefs) => void;
  onNotationChange: (notation: Notation, coalesceKey?: string) => void;
  onPlayheadChange: (beats: number) => void;
};

type DragState =
  | { kind: "pan"; lastX: number; lastY: number }
  | {
      kind: "move";
      noteIndex: number;
      originStart: number;
      originPitch: number;
      grabBeats: number;
      grabPitch: number;
    }
  | {
      kind: "resize";
      noteIndex: number;
      edge: "left" | "right";
      originStart: number;
      originDuration: number;
      grabBeats: number;
    };

function clamp(n: number, min: number, max: number): number {
  return Math.min(max, Math.max(min, n));
}

function pitchToY(semitone: number, offsetY: number, scale: number): number {
  const fromTop = PITCH_MAX - semitone;
  return RULER_HEIGHT + offsetY + fromTop * KEY_HEIGHT * scale;
}

function yToPitch(y: number, offsetY: number, scale: number): number {
  const local = (y - RULER_HEIGHT - offsetY) / (KEY_HEIGHT * scale);
  return Math.round(PITCH_MAX - local);
}

function beatsToX(beats: number, offsetX: number, scale: number): number {
  return PIANO_WIDTH + offsetX + beats * BEAT_WIDTH * scale;
}

function xToBeats(x: number, offsetX: number, scale: number): number {
  return (x - PIANO_WIDTH - offsetX) / (BEAT_WIDTH * scale);
}

type NoteHit =
  | { index: number; part: "body" | "left" | "right" }
  | null;

function hitTestNote(
  notation: Notation,
  x: number,
  y: number,
  offsetX: number,
  offsetY: number,
  scale: number,
): NoteHit {
  for (let i = notation.events.length - 1; i >= 0; i -= 1) {
    const note = notation.events[i]!;
    if (note.semitone < PITCH_MIN || note.semitone > PITCH_MAX) {
      continue;
    }
    const left = beatsToX(note.startBeats, offsetX, scale);
    const right = beatsToX(
      note.startBeats + note.durationBeats,
      offsetX,
      scale,
    );
    const top = pitchToY(note.semitone, offsetY, scale);
    const bottom = top + KEY_HEIGHT * scale;
    if (x < left || x > right || y < top || y > bottom) {
      continue;
    }
    if (x - left <= EDGE_PX) {
      return { index: i, part: "left" };
    }
    if (right - x <= EDGE_PX) {
      return { index: i, part: "right" };
    }
    return { index: i, part: "body" };
  }
  return null;
}

export function PianoRollPanel({
  notation,
  playheadBeats,
  prefs,
  audition,
  recording,
  toolbarExtra,
  onPrefsChange,
  onNotationChange,
  onPlayheadChange,
}: PianoRollPanelProps) {
  const recordingActive = recording?.active ?? false;
  const recordingActiveRef = useRef(recordingActive);
  recordingActiveRef.current = recordingActive;
  const takeNotes = recording?.takeNotes;
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const wrapRef = useRef<HTMLDivElement>(null);
  const [offsetX, setOffsetX] = useState(0);
  const [offsetY, setOffsetY] = useState(0);
  const [scale, setScale] = useState(1);
  const [selectedNote, setSelectedNote] = useState<number | null>(null);
  const [size, setSize] = useState({ w: 640, h: 360 });
  const dragRef = useRef<DragState | null>(null);
  const notationRef = useRef(notation);
  notationRef.current = notation;
  const prefsRef = useRef(prefs);
  prefsRef.current = prefs;
  const offsetRef = useRef({ x: offsetX, y: offsetY, scale });
  offsetRef.current = { x: offsetX, y: offsetY, scale };
  const selectedRef = useRef(selectedNote);
  selectedRef.current = selectedNote;
  const auditionRef = useRef(audition);
  auditionRef.current = audition;
  /** The auditioned note held by the current gesture (released on pointer up). */
  const heldNoteRef = useRef<number | null>(null);

  const startAudition = (semitone: number, velocity: number) => {
    const live = auditionRef.current;
    if (!live || !prefsRef.current.audition) {
      return;
    }
    live.noteOff(heldNoteRef.current);
    heldNoteRef.current = live.noteOn(semitone, velocity);
  };

  const retriggerAudition = (semitone: number, velocity: number) => {
    const live = auditionRef.current;
    if (live && heldNoteRef.current != null) {
      heldNoteRef.current = live.retrigger(heldNoteRef.current, semitone, velocity);
    }
  };

  const stopAudition = () => {
    auditionRef.current?.noteOff(heldNoteRef.current);
    heldNoteRef.current = null;
  };

  // Release a held audition if the panel goes away mid-gesture.
  useEffect(
    () => () => {
      auditionRef.current?.noteOff(heldNoteRef.current);
      heldNoteRef.current = null;
    },
    [],
  );

  useEffect(() => {
    const el = wrapRef.current;
    if (!el) {
      return;
    }
    const ro = new ResizeObserver((entries) => {
      const entry = entries[0];
      if (!entry) {
        return;
      }
      const { width, height } = entry.contentRect;
      setSize({
        w: Math.max(1, Math.floor(width)),
        h: Math.max(1, Math.floor(height)),
      });
    });
    ro.observe(el);
    return () => ro.disconnect();
  }, []);

  useEffect(() => {
    if (selectedNote != null && selectedNote >= notation.events.length) {
      setSelectedNote(null);
    }
  }, [notation.events.length, selectedNote]);

  const draw = useCallback(() => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    canvas.width = size.w * dpr;
    canvas.height = size.h * dpr;
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);

    const dark =
      getComputedStyle(canvas).getPropertyValue("color-scheme") === "dark" ||
      document.documentElement.getAttribute("data-mantine-color-scheme") ===
        "dark";
    const bg = dark ? "#1a1b1e" : "#f8f9fa";
    const grid = dark ? "#2c2e33" : "#dee2e6";
    const bar = dark ? "#495057" : "#adb5bd";
    const keyWhite = dark ? "#25262b" : "#ffffff";
    const keyBlack = dark ? "#101113" : "#212529";
    const keyBorder = dark ? "#373a40" : "#ced4da";
    const noteFill = dark ? "#4c6ef5" : "#4263eb";
    const noteSelected = dark ? "#ff922b" : "#f76707";
    const text = dark ? "#c1c2c5" : "#495057";
    const playhead = dark ? "#fa5252" : "#e03131";
    const takeFill = dark ? "#ff6b6b" : "#f03e3e";

    ctx.fillStyle = bg;
    ctx.fillRect(0, 0, size.w, size.h);

    // Piano lane keys
    for (let pitch = PITCH_MAX; pitch >= PITCH_MIN; pitch -= 1) {
      const y = pitchToY(pitch, offsetY, scale);
      const h = KEY_HEIGHT * scale;
      if (y + h < RULER_HEIGHT || y > size.h) {
        continue;
      }
      ctx.fillStyle = isBlackKey(pitch) ? keyBlack : keyWhite;
      ctx.fillRect(0, y, PIANO_WIDTH, h);
      ctx.strokeStyle = keyBorder;
      ctx.strokeRect(0, y, PIANO_WIDTH, h);
      if (!isBlackKey(pitch) && pitch % 12 === 0) {
        ctx.fillStyle = text;
        ctx.font = `${Math.max(9, 10 * scale)}px sans-serif`;
        ctx.fillText(semitoneLabel(pitch), 4, y + h * 0.72);
      }
    }

    // Grid background
    ctx.fillStyle = bg;
    ctx.fillRect(PIANO_WIDTH, RULER_HEIGHT, size.w - PIANO_WIDTH, size.h);

    const step = snapResolutionBeats(prefs.snapResolution);
    const beatsVisible = (size.w - PIANO_WIDTH - offsetX) / (BEAT_WIDTH * scale) + 2;
    const startBeat = Math.max(0, Math.floor(-offsetX / (BEAT_WIDTH * scale)) - 1);
    const endBeat = startBeat + beatsVisible + 2;

    for (let beat = startBeat; beat <= endBeat; beat += step) {
      const x = beatsToX(beat, offsetX, scale);
      if (x < PIANO_WIDTH || x > size.w) {
        continue;
      }
      const isBar = Math.abs(beat % 4) < 1e-9;
      ctx.strokeStyle = isBar ? bar : grid;
      ctx.beginPath();
      ctx.moveTo(x, RULER_HEIGHT);
      ctx.lineTo(x, size.h);
      ctx.stroke();
    }

    for (let pitch = PITCH_MIN; pitch <= PITCH_MAX; pitch += 1) {
      const y = pitchToY(pitch, offsetY, scale);
      if (y < RULER_HEIGHT || y > size.h) {
        continue;
      }
      ctx.strokeStyle = grid;
      ctx.beginPath();
      ctx.moveTo(PIANO_WIDTH, y);
      ctx.lineTo(size.w, y);
      ctx.stroke();
    }

    // Notes
    notation.events.forEach((note, index) => {
      if (note.semitone < PITCH_MIN || note.semitone > PITCH_MAX) {
        return;
      }
      const x = beatsToX(note.startBeats, offsetX, scale);
      const w = Math.max(2, note.durationBeats * BEAT_WIDTH * scale);
      const y = pitchToY(note.semitone, offsetY, scale);
      const h = KEY_HEIGHT * scale - 1;
      ctx.fillStyle = index === selectedNote ? noteSelected : noteFill;
      ctx.fillRect(x, y, w, h);
    });

    // Take in progress (recording): translucent, over the notation.
    if (takeNotes && takeNotes.length > 0) {
      ctx.globalAlpha = 0.55;
      ctx.fillStyle = takeFill;
      for (const note of takeNotes) {
        if (note.semitone < PITCH_MIN || note.semitone > PITCH_MAX) {
          continue;
        }
        const x = beatsToX(note.startBeats, offsetX, scale);
        const w = Math.max(2, note.durationBeats * BEAT_WIDTH * scale);
        const y = pitchToY(note.semitone, offsetY, scale);
        ctx.fillRect(x, y, w, KEY_HEIGHT * scale - 1);
      }
      ctx.globalAlpha = 1;
    }

    // Ruler strip
    ctx.fillStyle = dark ? "#25262b" : "#e9ecef";
    ctx.fillRect(PIANO_WIDTH, 0, size.w - PIANO_WIDTH, RULER_HEIGHT);
    ctx.fillStyle = dark ? "#1a1b1e" : "#f1f3f5";
    ctx.fillRect(0, 0, PIANO_WIDTH, RULER_HEIGHT);
    ctx.fillStyle = text;
    ctx.font = "10px sans-serif";
    for (let beat = Math.floor(startBeat); beat <= endBeat; beat += 1) {
      const x = beatsToX(beat, offsetX, scale);
      if (x < PIANO_WIDTH || x > size.w) {
        continue;
      }
      ctx.fillText(String(beat), x + 2, 14);
    }

    // Playhead
    const phX = beatsToX(playheadBeats, offsetX, scale);
    if (phX >= PIANO_WIDTH && phX <= size.w) {
      ctx.strokeStyle = playhead;
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(phX, 0);
      ctx.lineTo(phX, size.h);
      ctx.stroke();
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(phX - 5, 0);
      ctx.lineTo(phX + 5, 0);
      ctx.lineTo(phX, 8);
      ctx.closePath();
      ctx.fillStyle = playhead;
      ctx.fill();
    }
  }, [
    notation.events,
    offsetX,
    offsetY,
    playheadBeats,
    prefs.snapResolution,
    scale,
    selectedNote,
    size.h,
    size.w,
    takeNotes,
  ]);

  useEffect(() => {
    draw();
  }, [draw]);

  const clientToLocal = (clientX: number, clientY: number) => {
    const canvas = canvasRef.current!;
    const rect = canvas.getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top };
  };

  const onPointerDown = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (!canvas) {
      return;
    }
    canvas.setPointerCapture(event.pointerId);
    const { x, y } = clientToLocal(event.clientX, event.clientY);
    const { x: ox, y: oy, scale: sc } = offsetRef.current;
    const current = notationRef.current;
    const ui = prefsRef.current;

    if (event.button === 1 || (event.button === 0 && event.altKey)) {
      dragRef.current = { kind: "pan", lastX: event.clientX, lastY: event.clientY };
      return;
    }

    if (event.button !== 0) {
      return;
    }

    // The notation is read-only while a take records (pan still works).
    if (recordingActiveRef.current) {
      return;
    }

    // Ruler parks playhead
    if (y <= RULER_HEIGHT && x >= PIANO_WIDTH) {
      const beats = snapBeats(
        xToBeats(x, ox, sc),
        snapResolutionBeats(ui.snapResolution),
        ui.snapEnabled,
      );
      onPlayheadChange(Math.max(0, beats));
      return;
    }

    if (x < PIANO_WIDTH || y < RULER_HEIGHT) {
      return;
    }

    const hit = hitTestNote(current, x, y, ox, oy, sc);
    const step = snapResolutionBeats(ui.snapResolution);

    if (ui.tool === "draw") {
      if (hit) {
        return;
      }
      const pitch = clamp(yToPitch(y, oy, sc), PITCH_MIN, PITCH_MAX);
      const start = snapBeats(xToBeats(x, ox, sc), step, ui.snapEnabled);
      const next = applyNotationEdit(current, {
        kind: "draw",
        semitone: pitch,
        startBeats: start,
        durationBeats: step,
      });
      if (next !== current) {
        onNotationChange(next);
        setSelectedNote(next.events.length - 1);
        // Sounds while the button is held; released on pointer up.
        startAudition(pitch, next.events[next.events.length - 1]?.velocity ?? 1);
      }
      return;
    }

    if (ui.tool === "erase") {
      if (!hit) {
        return;
      }
      const next = applyNotationEdit(current, {
        kind: "erase",
        noteIndex: hit.index,
      });
      if (next !== current) {
        onNotationChange(next);
        setSelectedNote(null);
      }
      return;
    }

    // select
    if (!hit) {
      setSelectedNote(null);
      return;
    }
    setSelectedNote(hit.index);
    const note = current.events[hit.index]!;
    const grabBeats = xToBeats(x, ox, sc);
    if (hit.part === "left" || hit.part === "right") {
      dragRef.current = {
        kind: "resize",
        noteIndex: hit.index,
        edge: hit.part,
        originStart: note.startBeats,
        originDuration: note.durationBeats,
        grabBeats,
      };
    } else {
      dragRef.current = {
        kind: "move",
        noteIndex: hit.index,
        originStart: note.startBeats,
        originPitch: note.semitone,
        grabBeats,
        grabPitch: yToPitch(y, oy, sc),
      };
      startAudition(note.semitone, note.velocity);
    }
  };

  const onPointerMove = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const drag = dragRef.current;
    if (!drag) {
      return;
    }
    const { x: ox, y: oy, scale: sc } = offsetRef.current;
    const ui = prefsRef.current;
    const step = snapResolutionBeats(ui.snapResolution);

    if (drag.kind === "pan") {
      const dx = event.clientX - drag.lastX;
      const dy = event.clientY - drag.lastY;
      drag.lastX = event.clientX;
      drag.lastY = event.clientY;
      setOffsetX((v) => v + dx);
      setOffsetY((v) => v + dy);
      return;
    }

    const { x, y } = clientToLocal(event.clientX, event.clientY);
    const current = notationRef.current;

    if (drag.kind === "move") {
      const deltaBeats = xToBeats(x, ox, sc) - drag.grabBeats;
      const deltaPitch = yToPitch(y, oy, sc) - drag.grabPitch;
      let start = drag.originStart + deltaBeats;
      start = snapBeats(start, step, ui.snapEnabled);
      const pitch = clamp(
        drag.originPitch + deltaPitch,
        PITCH_MIN,
        PITCH_MAX,
      );
      const next = applyNotationEdit(current, {
        kind: "move",
        noteIndex: drag.noteIndex,
        startBeats: start,
        semitone: Math.round(pitch),
      });
      if (next !== current) {
        onNotationChange(next, `move:${drag.noteIndex}`);
      }
      // Moving in time keeps the voice; a new pitch retriggers it.
      retriggerAudition(
        Math.round(pitch),
        current.events[drag.noteIndex]?.velocity ?? 1,
      );
      return;
    }

    if (drag.kind === "resize") {
      const cursorBeats = xToBeats(x, ox, sc);
      const delta = cursorBeats - drag.grabBeats;
      let start = drag.originStart;
      let duration = drag.originDuration;
      if (drag.edge === "right") {
        duration = drag.originDuration + delta;
        duration = snapBeats(duration, step, ui.snapEnabled);
        duration = Math.max(step, duration);
      } else {
        const end = drag.originStart + drag.originDuration;
        start = snapBeats(drag.originStart + delta, step, ui.snapEnabled);
        start = Math.min(start, end - step);
        duration = end - start;
      }
      const next = applyNotationEdit(current, {
        kind: "resize",
        noteIndex: drag.noteIndex,
        startBeats: Math.max(0, start),
        durationBeats: duration,
      });
      if (next !== current) {
        onNotationChange(next, `resize:${drag.noteIndex}`);
      }
    }
  };

  const onPointerUp = (event: React.PointerEvent<HTMLCanvasElement>) => {
    const canvas = canvasRef.current;
    if (canvas?.hasPointerCapture(event.pointerId)) {
      canvas.releasePointerCapture(event.pointerId);
    }
    dragRef.current = null;
    stopAudition();
  };

  const onWheel = (event: React.WheelEvent<HTMLCanvasElement>) => {
    event.preventDefault();
    const { x, y } = clientToLocal(event.clientX, event.clientY);
    if (x < PIANO_WIDTH) {
      return;
    }
    const { x: ox, y: oy, scale: sc } = offsetRef.current;
    const factor = event.deltaY < 0 ? 1.1 : 1 / 1.1;
    const nextScale = clamp(sc * factor, SCALE_MIN, SCALE_MAX);
    const beats = xToBeats(x, ox, sc);
    const pitch = yToPitch(y, oy, sc);
    const nextOx = x - PIANO_WIDTH - beats * BEAT_WIDTH * nextScale;
    const nextOy =
      y - RULER_HEIGHT - (PITCH_MAX - pitch) * KEY_HEIGHT * nextScale;
    setScale(nextScale);
    setOffsetX(nextOx);
    setOffsetY(nextOy);
  };

  return (
    <div className={classes.root}>
      <div className={classes.toolbar}>
        <SegmentedControl
          size="xs"
          value={prefs.tool}
          onChange={(value) =>
            onPrefsChange({ ...prefs, tool: value as NotationTool })
          }
          data={[
            { label: "Draw", value: "draw" },
            { label: "Select", value: "select" },
            { label: "Erase", value: "erase" },
          ]}
        />
        <Group gap={6}>
          <Checkbox
            size="xs"
            label="Snap"
            checked={prefs.snapEnabled}
            onChange={(event) =>
              onPrefsChange({
                ...prefs,
                snapEnabled: event.currentTarget.checked,
              })
            }
          />
          <Checkbox
            size="xs"
            label="Audition"
            title={
              audition
                ? "Sound notes while placing, pressing, or dragging them"
                : "Audition needs a playback extension pack"
            }
            disabled={!audition}
            checked={prefs.audition}
            onChange={(event) =>
              onPrefsChange({
                ...prefs,
                audition: event.currentTarget.checked,
              })
            }
          />
          <Select
            size="xs"
            w={90}
            data={SNAP_RESOLUTIONS}
            value={prefs.snapResolution}
            onChange={(value) => {
              if (
                value === "1/4" ||
                value === "1/8" ||
                value === "1/16" ||
                value === "1/32"
              ) {
                onPrefsChange({ ...prefs, snapResolution: value });
              }
            }}
          />
        </Group>
        {recording ? (
          <Group gap={6}>
            <Button
              size="compact-xs"
              color="red"
              variant={recordingActive ? "filled" : "light"}
              title={
                recordingActive
                  ? "Stop recording and add the take (R)"
                  : "Record from the playhead (R)"
              }
              onClick={recording.onToggle}
            >
              {recordingActive
                ? recording.countIn
                  ? "● Count-in…"
                  : "■ Stop"
                : "● Record"}
            </Button>
            <Checkbox
              size="xs"
              label="Click"
              title="Metronome and one-bar count-in while recording"
              disabled={recordingActive}
              checked={prefs.click}
              onChange={(event) =>
                onPrefsChange({ ...prefs, click: event.currentTarget.checked })
              }
            />
          </Group>
        ) : null}
        {toolbarExtra}
        <Text size="xs" c="dimmed">
          D/S/E tools · N snap · R record · middle-drag pan · wheel zoom · ruler
          sets playhead
        </Text>
      </div>
      <div className={classes.canvasWrap} ref={wrapRef}>
        <canvas
          ref={canvasRef}
          className={classes.canvas}
          style={{
            width: size.w,
            height: size.h,
            cursor:
              prefs.tool === "draw"
                ? "crosshair"
                : prefs.tool === "erase"
                  ? "cell"
                  : "default",
          }}
          onPointerDown={onPointerDown}
          onPointerMove={onPointerMove}
          onPointerUp={onPointerUp}
          onPointerCancel={onPointerUp}
          onWheel={onWheel}
          onContextMenu={(event) => event.preventDefault()}
        />
      </div>
    </div>
  );
}
