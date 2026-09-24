export interface Note {
  name: string;
  text: string;
  tags: string[];
}

export type Store = Map<string, Note>;
