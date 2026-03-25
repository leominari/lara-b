# Lara B — WhatsApp AI Assistant

> Um assistente de IA flutuante que vive no seu desktop, sincroniza suas mensagens do WhatsApp e responde perguntas sobre elas em tempo real.

![Tauri](https://img.shields.io/badge/Tauri_2-24C8D8?logo=tauri&logoColor=white)
![Vue 3](https://img.shields.io/badge/Vue_3-42b883?logo=vue.js&logoColor=white)
![Rust](https://img.shields.io/badge/Rust-CE422B?logo=rust&logoColor=white)

---

## O que é

**Lara B** é um widget de desktop (macOS) construído com Tauri + Vue 3. Ele aparece como uma mascote gato animada flutuando diretamente na área de trabalho — sem janela, sem borda, só o gato.

A cada N minutos, ele abre o WhatsApp Web em segundo plano via Playwright, captura as mensagens mais recentes e as armazena localmente em SQLite. Você pode então clicar no gato e fazer perguntas sobre suas conversas — *"Tem algo urgente do João?"*, *"O que combinamos para sexta?"* — e receber respostas via streaming de um LLM à sua escolha.

---

## Funcionalidades

- **Mascote flutuante transparente** — só o gato visível na área de trabalho, sem chrome de aplicativo
- **Sincronização automática** do WhatsApp Web via Playwright (intervalo configurável)
- **Perguntas em linguagem natural** sobre suas mensagens com resposta em streaming
- **Balão de fala** com notificação de novas mensagens após cada sync
- **Múltiplos provedores LLM** — Claude (Anthropic), OpenAI, ou Ollama local
- **Controles ao hover** — mover (⠿), configurações (⚙), fechar (✕)
- **100% local** — mensagens ficam no seu Mac, nunca saem da máquina

---

## Stack

| Camada | Tecnologia |
|---|---|
| Desktop shell | Tauri 2 (Rust) |
| Frontend | Vue 3 + Composition API (`<script setup>`) + Vite |
| Animação | Lottie Web |
| Banco de dados | SQLite via `rusqlite` |
| Sync WhatsApp | Node.js + Playwright Chromium (stealth) |
| LLM | Claude API / OpenAI API / Ollama (streaming SSE) |

---

## Arquitetura

```
┌─────────────────────────────────────────┐
│              Desktop (macOS)            │
│                                         │
│   ┌─────────────┐                       │
│   │  Gato Lottie│ ← janela transparente │
│   │  320 × 320  │   sempre no topo      │
│   └──────┬──────┘                       │
│          │ clique                        │
│   ┌──────▼──────────────────────────┐   │
│   │  Balão de fala + Input bar      │   │
│   └──────────────────────────────┬──┘   │
│                                  │ invoke│
│   ┌──────────────────────────────▼──┐   │
│   │  Rust backend (Tauri commands)  │   │
│   │  • ask_question → LLM stream    │   │
│   │  • get/save_settings → SQLite   │   │
│   │  • sync scheduler (tokio)       │   │
│   └────────────────┬────────────────┘   │
│                    │ spawn              │
│   ┌────────────────▼────────────────┐   │
│   │  Node.js + Playwright           │   │
│   │  Abre WhatsApp Web, raspa msgs  │   │
│   │  Salva JSON → SQLite            │   │
│   └─────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

---

## Pré-requisitos

- macOS (testado em macOS 15+)
- [Rust + Cargo](https://rustup.rs/)
- [Node.js 18+](https://nodejs.org/)
- [Tauri CLI](https://v2.tauri.app/start/prerequisites/)

---

## Instalação

### 1. Clone o repositório

```bash
git clone git@github.com:leominari/lara-b.git
cd lara-b/focus-widget
```

### 2. Instale as dependências do frontend

```bash
npm install
```

### 3. Instale as dependências do script de sync

```bash
cd scripts
npm install
npx playwright install chromium
cd ..
```

### 4. Execute em modo desenvolvimento

```bash
npm run tauri dev
```

Na primeira execução, o Setup Wizard vai verificar se Node.js e Playwright estão disponíveis. Se tudo estiver ok, o gato aparece e você pode escanear o QR do WhatsApp Web para autenticar.

---

## Configurações

Passe o mouse sobre o gato e clique em ⚙ para abrir as configurações.

| Campo | Descrição | Padrão |
|---|---|---|
| Intervalo de sync | Com que frequência sincronizar as mensagens | 5 min |
| Histórico inicial | Quantos dias de histórico importar na primeira vez | 7 dias |
| Provedor LLM | `claude`, `openai`, ou `ollama` | `claude` |
| API Key | Chave da API (Claude ou OpenAI) | — |
| Ollama URL | Endereço do servidor Ollama local | `http://localhost:11434` |
| Modelo Ollama | Nome do modelo | `llama3` |
| Tempo do balão | Segundos até o balão de notificação sumir | 10 s |

Todas as configurações são salvas localmente em SQLite.

---

## Como usar

1. **Clique no gato** para abrir o input
2. **Digite sua pergunta** — ex: *"Tem mensagem urgente hoje?"*
3. **Pressione Enter** — a resposta aparece no balão em tempo real
4. **Pressione Esc** ou clique fora para fechar o input
5. **Arraste** pelo ícone ⠿ para reposicionar o gato na tela

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
│   │   ├── WhatsAppAssistant.vue   # UI principal (gato + balão + input)
│   │   ├── CatMascot.vue           # Animação Lottie
│   │   ├── SettingsPanel.vue       # Painel de configurações
│   │   └── SetupWizard.vue         # Wizard de primeira execução
│   └── composables/
│       └── useAssistant.ts         # Estado: sync, LLM, bubble, settings
├── src-tauri/src/                  # Backend Rust
│   ├── commands.rs                 # Tauri commands (ask, settings, check_qr)
│   ├── db.rs                       # SQLite: mensagens + settings
│   ├── sync.rs                     # Scheduler de sync + parser do script Node
│   ├── query.rs                    # Prompt builder para o LLM
│   └── llm/                        # Providers: Claude, OpenAI, Ollama (SSE)
├── scripts/
│   └── sync.js                     # Script Node.js + Playwright (WhatsApp Web)
└── public/
    └── loader-cat.json             # Animação Lottie do gato
```

---

## Privacidade e segurança

- Nenhuma mensagem é enviada para servidores externos além do LLM configurado (somente o contexto relevante para responder sua pergunta)
- A sessão do WhatsApp Web é armazenada localmente em `~/.whatsapp-assistant/profile/` e nunca comitada no repositório
- O banco SQLite fica no diretório de dados do app no macOS e também nunca vai para o repositório

---

## Licença

MIT
