/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_OPENAI_API_KEY: string;
  // dodaj tutaj inne zmienne jeśli zajdzie potrzeba
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
