export interface Memory {
  id: number
  content: string
  md_file_path: string | null
  created_at: string
  tags: string | null
}

export interface MdRecord {
  frontmatter: {
    created: string
    tags: string[] | null
    entities: string[] | null
  }
  content: string
  file_path: string
}
