# Lara B — WhatsApp AI Assistant

> Um assistente de IA flutuante que vive no seu desktop, sincroniza suas mensagens do WhatsApp em tempo real e responde perguntas sobre elas via streaming.

![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?logo=tauri&logoColor=white)
![Vue 3](https://img.shields.io/badge/Vue_3-42b883?logo=vue.js&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white)

---

## O que é

**Lara B** é um widget de desktop (macOS) construído com Tauri + Vue 3. Ele aparece como uma mascote gato animada flutuando diretamente na área de trabalho — sem janela, sem borda, só o gato.

Via **Baileys** (WebSocket nativo do WhatsApp), ele recebe mensagens em tempo real e as armazena localmente em SQLite. Você pode clicar no gato e fazer perguntas sobre suas conversas — *"Tem algo urgente do João?"*, *"O que combinamos para sexta?"* — e receber respostas via streaming de um LLM à sua escolha.

Contatos marcados como **favoritos** recebem tratamento especial: quando enviam mensagens, o gato automaticamente resume o que estão falando após 2 minutos (debounce), sem você precisar perguntar.

---

## Funcionalidades

- **Mascote flutuante transparente** — só o gato visível na área de trabalho, janela 320×400 sempre no topo
- **Sincronização em tempo real** via Baileys (WebSocket — sem Playwright, sem scraping)
- **Perguntas em linguagem natural** sobre suas mensagens com resposta em streaming
- **Balão de fala paginado** — respostas longas divididas por sentenças com navegação ◀ ▶
- **Renderização markdown** no balão (negrito, itálico, listas, código)
- **Favoritos** (★) — contatos VIP recebem resumo automático após 2 min de silêncio
- **Múltiplos provedores LLM** — Claude (Anthropic), OpenAI, ou Ollama local (padrão)
- **Controles ao hover** — mover (⠿), favoritos (★), configurações (⚙), fechar (✕)
- **100% local** — mensagens ficam no seu Mac, nunca saem da máquina

---

## Stack

| Camada | Tecnologia |
|---|---|
| Desktop shell | Tauri 2 (Rust) |
| Frontend | Vue 3 + Composition API (`<script setup>`) + Vite |
| Animação | Lottie Web |
| Banco de dados | SQLite via `rusqlite` |
| Sync WhatsApp | Node.js + Baileys (`@whiskeysockets/baileys`) |
| Markdown | `marked` |
| LLM | Claude API / OpenAI API / Ollama (streaming) |

---

## Arquitetura

```
┌─────────────────────────────────────────────┐
│                Desktop (macOS)              │
│                                             │
│   ┌─────────────┐                           │
│   │  Gato Lottie│ ← janela 320×400          │
│   │  + balão MD │   transparente, no topo   │
│   └──────┬──────┘                           │
│          │ clique / favorito                 │
│   ┌──────▼──────────────────────────────┐   │
│   │  Rust backend (Tauri commands)      │   │
│   │  • ask_question → LLM stream        │   │
│   │  • summarize_contact → LLM stream   │   │
│   │  • add/remove_favorite → SQLite     │   │
│   │  • get/save_settings → SQLite       │   │
│   └────────────────┬────────────────────┘   │
│                    │ spawn (persistent)      │
│   ┌────────────────▼────────────────────┐   │
│   │  Node.js + Baileys                  │   │
│   │  WebSocket WhatsApp → JSON lines    │   │
│   │  QR na primeira vez, persiste auth  │   │
│   └─────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

**IPC Rust ↔ Node**: stdout JSON lines (`type: messages | qr | ready | logout | error`)

**Eventos Tauri → Vue**: `sync_start`, `sync_complete`, `qr_required`, `llm_token`, `llm_done`, `llm_error`, `favorite_activity`

---

## Pré-requisitos

- macOS (testado em macOS 15+)
- [Rust + Cargo](https://rustup.rs/)
- [Node.js 18+](https://nodejs.org/)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)
- Ollama rodando localmente (ou chave de API Claude/OpenAI)

---

## Instalação

### 1. Clone o repositório

```bash
git clone git@github.com:leominari/lara-b.git
cd lara-b/focus-widget
```

### 2. Instale as dependências

```bash
npm install          # frontend
cd scripts && npm install && cd ..   # baileys
```

### 3. Execute em modo desenvolvimento

```bash
npm run tauri dev
```

Na primeira execução, o Setup Wizard verifica se Node.js está disponível. Em seguida, o Baileys gera um QR code — escaneie com o WhatsApp no celular para autenticar. A sessão fica salva em `~/.whatsapp-assistant/baileys-auth/`.

---

## Configurações

Passe o mouse sobre o gato e clique em ⚙ para abrir as configurações.

| Campo | Descrição | Padrão |
|---|---|---|
| Intervalo de sync | Frequência de sincronização | 5 min |
| Histórico inicial | Dias de histórico na primeira vez | 7 dias |
| Provedor LLM | `ollama`, `claude` ou `openai` | `ollama` |
| API Key | Chave da API (Claude ou OpenAI) | — |
| Ollama URL | Endereço do servidor Ollama local | `http://localhost:11434` |
| Modelo Ollama | Nome do modelo | `llama3` |
| Tempo do balão | Segundos até o balão sumir | 10 s |

---

## Favoritos

Clique em ★ nos controles (ao passar o mouse) para abrir o painel de favoritos.

- **Busque** pelo nome do contato
- **Clique na estrela** para favoritar/desfavoritar
- Quando um favorito enviar mensagens, o gato aguarda **2 minutos** (debounce) e então gera automaticamente um resumo do que a pessoa está falando — sem você precisar perguntar

---

## Como usar

1. **Clique no gato** para abrir o input
2. **Digite sua pergunta** — ex: *"Tem mensagem urgente hoje?"*
3. **Pressione Enter** — a resposta aparece no balão paginado em tempo real
4. Use **◀ ▶** para navegar entre páginas da resposta
5. Clique **✕** no balão para fechá-lo
6. **Pressione Esc** ou clique fora para fechar o input
7. **Arraste** pelo ícone ⠿ para reposicionar

---

## Build para produção

```bash
npm run tauri build
```

O app compilado fica em `src-tauri/target/release/bundle/`.

---

## Estrutura do projeto

```
focus-widget/
├── src/                            # Frontend Vue 3
│   ├── components/
│   │   ├── WhatsAppAssistant.vue   # UI principal (gato + balão + input + controles)
│   │   ├── CatMascot.vue           # Animação Lottie
│   │   ├── SettingsPanel.vue       # Painel de configurações (overlay)
│   │   ├── FavoritesPanel.vue      # Painel de favoritos (overlay)
│   │   └── SetupWizard.vue         # Wizard de primeira execução
│   └── composables/
│       └── useAssistant.ts         # Estado: sync, LLM, bubble, settings, favorites
├── src-tauri/src/                  # Backend Rust
│   ├── commands.rs                 # Tauri commands
│   ├── db.rs                       # SQLite: messages, settings, favorites
│   ├── sync.rs                     # Processo Baileys persistente + parser IPC
│   ├── query.rs                    # Prompt builders (geral + por contato)
│   └── llm/                        # Providers: Claude, OpenAI, Ollama
├── scripts/
│   ├── baileys.js                  # Processo Node.js persistente (WebSocket WhatsApp)
│   └── package.json
└── public/
    └── loader-cat.json             # Animação Lottie do gato
```

---

## Privacidade e segurança

- Nenhuma mensagem é enviada para servidores externos além do LLM configurado (somente o contexto relevante para responder sua pergunta)
- A sessão do WhatsApp fica em `~/.whatsapp-assistant/baileys-auth/` — nunca comitada
- O banco SQLite fica no diretório de dados do app no macOS — nunca vai para o repositório

---

## Licença

MIT
