import { useMemo } from "react";
import { Text, useMantineColorScheme } from "@mantine/core";
import {
  Background,
  Controls,
  ReactFlow,
  ReactFlowProvider,
  type Edge,
  type Node,
} from "@xyflow/react";
import type { Graph } from "imp-core-types";
import { impToReactFlow } from "imp-graph-view";
import "@xyflow/react/dist/style.css";
import classes from "./ImpGraphCanvas.module.css";

const GRID_COLS = 4;
const CELL_W = 220;
const CELL_H = 120;

type ImpGraphCanvasProps = {
  graph: Graph;
};

/** Host-only grid so Imp→RF default `{0,0}` positions do not stack. */
function applyGridPositions(nodes: Node[]): Node[] {
  return nodes.map((node, index) => ({
    ...node,
    // Clear Imp NodeTypeId so RF uses the built-in default node (Tome registers customs).
    type: undefined,
    position: {
      x: (index % GRID_COLS) * CELL_W,
      y: Math.floor(index / GRID_COLS) * CELL_H,
    },
    data: {
      ...node.data,
      label: node.type || node.id,
    },
  }));
}

function ImpGraphCanvasInner({ graph }: ImpGraphCanvasProps) {
  const { colorScheme } = useMantineColorScheme();
  const { nodes, edges } = useMemo(() => {
    const converted = impToReactFlow(graph);
    return {
      nodes: applyGridPositions(converted.nodes as Node[]),
      edges: converted.edges as Edge[],
    };
  }, [graph]);

  const empty = nodes.length === 0;
  const rfColorMode = colorScheme === "light" ? "light" : "dark";

  return (
    <div className={classes.root}>
      {empty ? (
        <div className={classes.empty}>
          <Text size="sm" c="dimmed" ta="center">
            Empty Imp graph
          </Text>
        </div>
      ) : (
        <ReactFlow
          className={classes.flow}
          nodes={nodes}
          edges={edges}
          colorMode={rfColorMode}
          fitView
          fitViewOptions={{ padding: 0.2 }}
          minZoom={0.1}
          nodesDraggable={false}
          nodesConnectable={false}
          elementsSelectable={false}
          edgesFocusable={false}
          nodesFocusable={false}
          panOnDrag
          zoomOnScroll
          proOptions={{ hideAttribution: true }}
        >
          <Background gap={16} size={1} />
          <Controls showInteractive={false} />
        </ReactFlow>
      )}
    </div>
  );
}

export function ImpGraphCanvas(props: ImpGraphCanvasProps) {
  return (
    <ReactFlowProvider>
      <ImpGraphCanvasInner {...props} />
    </ReactFlowProvider>
  );
}
