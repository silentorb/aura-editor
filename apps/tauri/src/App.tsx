import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AppShell,
  Badge,
  Box,
  Button,
  Group,
  SegmentedControl,
  Stack,
  Tabs,
  Text,
  Title,
  useMantineColorScheme,
} from "@mantine/core";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import {
  CMD_DOCUMENT_CLOSE_ACTIVE,
  CMD_DOCUMENT_NEW_GRAPH,
  CMD_DOCUMENT_NEW_NOTATION,
  CMD_DOCUMENT_OPEN,
  CMD_DOCUMENT_OPEN_FILE,
  CMD_DOCUMENT_OPEN_WORKSPACE,
  CMD_DOCUMENT_SAVE,
  CMD_PLAY_START,
  CMD_PLAY_STOP,
  CMD_VIEW_TOGGLE_COLOR_SCHEME,
  EDITOR_COMMAND_EVENT,
  type EditorCommandPayload,
} from "./commands";
import {
  documentFromDto,
  emptyImpGraph,
  newUntitledDocument,
  type DocumentSummaryDto,
  type EditorDocument,
  type TransportState,
} from "./documents";
import { ImpGraphCanvas } from "./ImpGraphCanvas";
import classes from "./App.module.css";

export default function App() {
  const { colorScheme, setColorScheme, toggleColorScheme } =
    useMantineColorScheme();
  const [documents, setDocuments] = useState<EditorDocument[]>([]);
  const [activeId, setActiveId] = useState("");
  const [transport, setTransport] = useState<TransportState>("stopped");
  const [status, setStatus] = useState(
    "Prototype · Open File… bridged · playback still mock",
  );

  const active = useMemo(
    () => documents.find((doc) => doc.id === activeId) ?? documents[0],
    [documents, activeId],
  );

  const openFile = useCallback(async () => {
    try {
      const result = await invoke<DocumentSummaryDto | null>(
        "pick_and_open_document",
      );
      if (!result) {
        setStatus("Open File… cancelled");
        return;
      }
      const doc = documentFromDto(result);
      setDocuments((prev) => {
        const existing = prev.find((d) => d.origin === doc.origin);
        if (existing) {
          queueMicrotask(() => {
            setActiveId(existing.id);
            setStatus(`Activated · ${existing.title}`);
          });
          return prev;
        }
        queueMicrotask(() => {
          setActiveId(doc.id);
          setStatus(`Opened · ${doc.title}`);
        });
        return [...prev, doc];
      });
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setStatus(`Open failed · ${message}`);
    }
  }, []);

  const closeActive = useCallback(() => {
    setDocuments((prev) => {
      if (prev.length === 0) {
        queueMicrotask(() => setStatus("No document to close"));
        return prev;
      }
      const currentId = activeId || prev[0]?.id;
      const next = prev.filter((d) => d.id !== currentId);
      const nextActive = next[0]?.id ?? "";
      queueMicrotask(() => {
        setActiveId(nextActive);
        setStatus(next.length ? "Closed tab" : "Closed last tab");
        if (next.length === 0) {
          setTransport("stopped");
        }
      });
      return next;
    });
  }, [activeId]);

  const dispatchCommand = useCallback(
    (id: string) => {
      switch (id) {
        case CMD_DOCUMENT_OPEN_FILE:
          void openFile();
          break;
        case CMD_DOCUMENT_OPEN_WORKSPACE:
          setStatus("Open Workspace… deferred — Miro Session 3+");
          break;
        case CMD_DOCUMENT_OPEN:
          setStatus("Open… deferred — Miro NodeId Session 3+");
          break;
        case CMD_DOCUMENT_SAVE:
          setStatus("Save deferred — editable models not hosted yet");
          break;
        case CMD_DOCUMENT_NEW_GRAPH: {
          const doc = newUntitledDocument("imp-graph");
          setDocuments((prev) => [...prev, doc]);
          setActiveId(doc.id);
          setStatus(`New graph · ${doc.title}`);
          break;
        }
        case CMD_DOCUMENT_NEW_NOTATION: {
          const doc = newUntitledDocument("notation");
          setDocuments((prev) => [...prev, doc]);
          setActiveId(doc.id);
          setStatus(`New notation · ${doc.title}`);
          break;
        }
        case CMD_DOCUMENT_CLOSE_ACTIVE:
          closeActive();
          break;
        case CMD_PLAY_START: {
          setTransport("playing");
          setStatus(
            active
              ? `Playing (mock) · ${active.title}`
              : "Playing (mock) · no document",
          );
          break;
        }
        case CMD_PLAY_STOP: {
          setTransport("stopped");
          setStatus("Stopped (mock)");
          break;
        }
        case CMD_VIEW_TOGGLE_COLOR_SCHEME:
          toggleColorScheme();
          break;
        default:
          setStatus(`Unhandled command · ${id}`);
          break;
      }
    },
    [active, closeActive, openFile, toggleColorScheme],
  );

  useEffect(() => {
    let cancelled = false;
    let unlisten: (() => void) | undefined;

    void listen<EditorCommandPayload>(EDITOR_COMMAND_EVENT, (event) => {
      if (!cancelled) {
        dispatchCommand(event.payload.id);
      }
    }).then((fn) => {
      if (cancelled) {
        fn();
      } else {
        unlisten = fn;
      }
    });

    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [dispatchCommand]);

  return (
    <AppShell
      header={{ height: 40 }}
      footer={{ height: 28 }}
      aside={{ width: 280, breakpoint: "sm" }}
      padding={0}
      className={classes.shell}
    >
      <AppShell.Header className={classes.header}>
        <Group h="100%" px="sm" justify="space-between" gap="sm" wrap="nowrap">
          <Group gap="xs" wrap="nowrap">
            <Text size="sm" fw={600}>
              Aura Editor
            </Text>
            <Badge size="xs" variant="light" color="gray">
              Tauri experiment
            </Badge>
          </Group>
          <Group gap={6} wrap="nowrap">
            <SegmentedControl
              size="xs"
              value={colorScheme === "auto" ? "dark" : colorScheme}
              onChange={(value) =>
                setColorScheme(value === "light" ? "light" : "dark")
              }
              data={[
                { label: "Dark", value: "dark" },
                { label: "Light", value: "light" },
              ]}
            />
            <Button
              variant="default"
              onClick={() => dispatchCommand(CMD_DOCUMENT_OPEN_FILE)}
            >
              Open…
            </Button>
            <Button
              variant={transport === "playing" ? "filled" : "default"}
              color="green"
              onClick={() => dispatchCommand(CMD_PLAY_START)}
              disabled={transport === "playing"}
            >
              Play
            </Button>
            <Button
              variant="default"
              onClick={() => dispatchCommand(CMD_PLAY_STOP)}
              disabled={transport === "stopped"}
            >
              Stop
            </Button>
          </Group>
        </Group>
      </AppShell.Header>

      <AppShell.Aside className={classes.aside} p="sm">
        <Stack gap="xs">
          <Title order={6}>Properties</Title>
          {active ? (
            <Stack gap={4}>
              {active.properties.map((row) => (
                <Group
                  key={row.label}
                  gap="xs"
                  justify="space-between"
                  wrap="nowrap"
                >
                  <Text size="xs" c="dimmed">
                    {row.label}
                  </Text>
                  <Text size="xs" ta="right" lineClamp={1}>
                    {row.value}
                  </Text>
                </Group>
              ))}
            </Stack>
          ) : (
            <Text size="xs" c="dimmed">
              No selection
            </Text>
          )}
        </Stack>
      </AppShell.Aside>

      <AppShell.Main className={classes.main}>
        {documents.length === 0 ? (
          <Box className={classes.emptyState}>
            <Stack gap="xs" align="center">
              <Text size="sm" c="dimmed" ta="center">
                No documents open
              </Text>
              <Text size="xs" c="dimmed" ta="center">
                File → Open File… or New notation / New graph
              </Text>
              <Button
                variant="default"
                onClick={() => dispatchCommand(CMD_DOCUMENT_OPEN_FILE)}
              >
                Open File…
              </Button>
            </Stack>
          </Box>
        ) : (
          <Tabs
            value={activeId || documents[0]?.id}
            onChange={(value) => value && setActiveId(value)}
            className={classes.tabs}
          >
            <Tabs.List className={classes.tabList}>
              {documents.map((doc) => (
                <Tabs.Tab key={doc.id} value={doc.id}>
                  {doc.dirty ? `${doc.title} *` : doc.title}
                </Tabs.Tab>
              ))}
            </Tabs.List>

            {documents.map((doc) => (
              <Tabs.Panel key={doc.id} value={doc.id} className={classes.panel}>
                <Box p="md" className={classes.content}>
                  <Group gap="xs" mb="sm">
                    <Badge size="sm" variant="outline">
                      {doc.kind}
                    </Badge>
                    <Text size="xs" c="dimmed" lineClamp={1}>
                      {doc.origin}
                    </Text>
                  </Group>
                  <Title order={4} mb="xs">
                    {doc.title}
                  </Title>
                  <Text size="sm">{doc.summary}</Text>
                  {doc.kind === "imp-graph" ? (
                    <Box className={classes.canvas}>
                      <ImpGraphCanvas graph={doc.graph ?? emptyImpGraph()} />
                    </Box>
                  ) : (
                    <Box mt="lg" className={classes.canvasPlaceholder}>
                      <Text size="sm" c="dimmed" ta="center">
                        piano-roll canvas placeholder
                      </Text>
                    </Box>
                  )}
                </Box>
              </Tabs.Panel>
            ))}
          </Tabs>
        )}
      </AppShell.Main>

      <AppShell.Footer className={classes.footer}>
        <Group h="100%" px="sm" justify="space-between" wrap="nowrap">
          <Text size="xs" lineClamp={1}>
            {status}
          </Text>
          <Text size="xs" c="dimmed">
            {transport === "playing" ? "▶ playing (mock)" : "■ stopped"}
          </Text>
        </Group>
      </AppShell.Footer>
    </AppShell>
  );
}
