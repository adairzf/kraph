export interface GraphNode {
  id: string
  name: string
  type: string
  attributes: string | null
}

export interface GraphLink {
  source: string
  target: string
  relation: string
  strength: number
}

export interface GraphData {
  nodes: GraphNode[]
  links: GraphLink[]
}
