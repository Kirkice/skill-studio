import { memo, useEffect } from 'react';
import { ReactFlow, Background, Controls, MiniMap, useNodesState, type Node, type Edge } from '@xyflow/react';

type Props={nodes:Node[];edges:Edge[];onMove:(id:string,position:{x:number;y:number})=>void;onSelect:(id:string)=>void;onReference:(edge:Edge)=>void};
// Position updates stay inside this component. The editor/explorer and node metadata
// are not rebuilt for every pointer event; persistence happens only on drag stop.
export const GraphCanvas=memo(function GraphCanvas({nodes:initialNodes,edges,onMove,onSelect,onReference}:Props){
 const [nodes,setNodes,onNodesChange]=useNodesState(initialNodes);
 useEffect(()=>{setNodes(initialNodes);},[initialNodes,setNodes]);
 return <ReactFlow nodes={nodes} edges={edges} onNodesChange={onNodesChange} nodesDraggable nodesConnectable={false} fitView minZoom={0.15} maxZoom={2} onNodeDragStop={(_,node)=>onMove(node.id,node.position)} onNodeClick={(_,node)=>onSelect(node.id)} onEdgeClick={(_,edge)=>onReference(edge)}>
  <Background color="#293449" gap={24}/><Controls/><MiniMap nodeColor={node=>String(node.style?.borderColor??'#64748b')} maskColor="rgba(10,15,25,.7)"/>
 </ReactFlow>;
});
