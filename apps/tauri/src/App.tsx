import { useMemo, useState } from "react";
import {
  AppShell,
  Badge,
  Box,
  Button,
  Group,
  Stack,
  Tabs,
  Text,
  Title,
} from "@mantine/core";
import {
  MOCK_DOCUMENTS,
  type MockDocument,
  type TransportState,
} from "./mock/workspace";
import classes from "./App.module.css";

export default function App() {
  const [documents] = useState<MockDocument[]>(MOCK_DOCUMENTS);
  const [activeId, setActiveId] = useState(documents[0]?.id ?? "");
  const [transport, setTransport] = useState<TransportState>("stopped");
  const [status, setStatus] = useState("Prototype · mock workspace (no host audio)");

  const active = useMemo(
    () => documents.find((doc) => doc.id === activeId) ?? documents[0],
    [documents, activeId],
  );

  const onOpen = () => {
    setStatus("Open… mocked — filesystem/Miro wiring deferred");
  };

  const onPlay = () => {
    setTransport("playing");
    setStatus(
      active
        ? `Playing (mock) · ${active.title}`
        : "Playing (mock) · no document",
    );
  };

  const onStop = () => {
    setTransport("stopped");
    setStatus("Stopped (mock)");
  };

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
            <Button variant="default" onClick={onOpen}>
              Open…
            </Button>
            <Button
              variant={transport === "playing" ? "filled" : "default"}
              color="green"
              onClick={onPlay}
              disabled={transport === "playing"}
            >
              Play
            </Button>
            <Button
              variant="default"
              onClick={onStop}
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
                <Group key={row.label} gap="xs" justify="space-between" wrap="nowrap">
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
        <Tabs
          value={activeId}
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
                <Box mt="lg" className={classes.canvasPlaceholder}>
                  <Text size="sm" c="dimmed" ta="center">
                    {doc.kind === "imp-graph"
                      ? "graph-dag canvas placeholder"
                      : "piano-roll canvas placeholder"}
                  </Text>
                </Box>
              </Box>
            </Tabs.Panel>
          ))}
        </Tabs>
      </AppShell.Main>

      <AppShell.Footer className={classes.footer}>
        <Group h="100%" px="sm" justify="space-between" wrap="nowrap">
          <Text size="xs" lineClamp={1}>
            {status}
          </Text>
          <Text size="xs" c="dimmed">
            {transport === "playing" ? "▶ playing" : "■ stopped"}
          </Text>
        </Group>
      </AppShell.Footer>
    </AppShell>
  );
}
