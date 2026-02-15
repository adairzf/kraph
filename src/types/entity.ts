export interface Entity {
  id: number
  type: string
  name: string
  attributes: string | null
  created_at: string
  updated_at: string
}

export interface Relation {
  id: number
  from_entity_id: number
  to_entity_id: number
  relation_type: string
  strength: number
  created_at: string | null
}
