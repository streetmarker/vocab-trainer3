// src/utils/CategorizationAgent.ts
import { api } from "../hooks/useTauri";
import type { Word } from "../types";

export interface AgentStatus {
  isProcessing: boolean;
  progress: number;
  total: number;
  currentWord?: string;
  lastError?: string;
}

export class CategorizationAgent {
  private static instance: CategorizationAgent;
  private status: AgentStatus = { isProcessing: false, progress: 0, total: 0 };
  private onStatusChange?: (status: AgentStatus) => void;

  private constructor() {}

  public static getInstance(): CategorizationAgent {
    if (!CategorizationAgent.instance) {
      CategorizationAgent.instance = new CategorizationAgent();
    }
    return CategorizationAgent.instance;
  }

  public setStatusListener(cb: (s: AgentStatus) => void) {
    this.onStatusChange = cb;
  }

  private updateStatus(patch: Partial<AgentStatus>) {
    this.status = { ...this.status, ...patch };
    this.onStatusChange?.(this.status);
  }

  /**
   * Główna pętla Agenta
   */
  public async run() {
    if (this.status.isProcessing) return;

    try {
      this.updateStatus({ isProcessing: true, lastError: undefined });

      // 1. ANALYSE: Pobierz słowa bez kategorii
      const { words, categories } = await api.reclassifyWords();
      
      if (words.length === 0) {
        this.updateStatus({ isProcessing: false, progress: 0, total: 0 });
        return;
      }

      this.updateStatus({ total: words.length, progress: 0 });

      // 2. STRATEGY: Batching (grupy po 10 słów, by oszczędzić tokeny)
      const batchSize = 10;
      for (let i = 0; i < words.length; i += batchSize) {
        const batch = words.slice(i, i + batchSize);
        await this.processBatch(batch, categories);
        this.updateStatus({ progress: i + batch.length });
      }

      this.updateStatus({ isProcessing: false });
    } catch (err: any) {
      this.updateStatus({ isProcessing: false, lastError: err.toString() });
    }
  }

  private async processBatch(batch: Word[], allowedCategories: string[]) {
    const apiKey = import.meta.env.VITE_OPENAI_API_KEY;
    
    if (!apiKey) {
      throw new Error("Brak klucza API OpenAI (VITE_OPENAI_API_KEY) w pliku .env");
    }

    this.updateStatus({ currentWord: "Komunikacja z OpenAI..." });

    try {
      const response = await fetch("https://api.deepseek.com/v1/chat/completions", {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          "Authorization": `Bearer ${apiKey}`
        },
        body: JSON.stringify({
          model: "deepseek-chat", // Szybki i tani model idealny do kategoryzacji
          messages: [
            {
              role: "system",
              content: `Jesteś ekspertem lingwistyki. Przypisz każde z podanych słów do DOKŁADNIE JEDNEJ kategorii z poniższej listy:
              [${allowedCategories.join(", ")}]. 
              Zwróć wynik DOKŁADNIE w formacie JSON: {"results": [{"id": word_id, "category": "nazwa_kategorii"}]}.
              Jeśli słowo nie pasuje do żadnej kategorii, użyj 'bez kategorii'.`
            },
            {
              role: "user",
              content: JSON.stringify(batch.map(w => ({ id: w.id, term: w.term, definition: w.definition })))
            }
          ],
          response_format: { type: "json_object" }
        })
      });

      if (!response.ok) {
        const errData = await response.json();
        throw new Error(errData.error?.message || "Błąd API OpenAI");
      }

      const data = await response.json();
      const aiResults = JSON.parse(data.choices[0].message.content).results;

      // 3. EXECUTE: Aktualizacja bazy na podstawie odpowiedzi AI
      for (const result of aiResults) {
        const word = batch.find(w => w.id === result.id);
        if (word) {
          this.updateStatus({ currentWord: word.term });
          await api.updateWordCategory(word.id, result.category);
        }
      }

    } catch (err: any) {
      console.error("Agent OpenAI Error:", err);
      throw err;
    }
  }

}
