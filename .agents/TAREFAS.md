# Quadro de tarefas

Estados possíveis: `por fazer`, `em curso`, `bloqueada`, `em revisão`, `feito`.
O B passa as tarefas para `em revisão` quando acaba. O A revê, faz o merge e só então marca `feito`.

| ID | Dono | Tarefa | Estado |
|---|---|---|---|
| A1 | A | Tema `theme.rs` | feito |
| A2 | A | Modo de captura PNG | feito |
| A3 | A | Componentes `widgets.rs` | feito |
| A4 | A | `ARQUITETURA.md` + separar `main_page.rs` e o teclado da biblioteca | feito |
| A6 | A | API da playlist (`previous`, `has_next`, `position_label`…) | feito |
| A7 | A | Favoritos, recentes e `lib_cursor` na `Library`/`UiState` (para B5/B6) | feito |
| A8 | A | Rever e fazer merge do trabalho do B, reinstalar a app | contínuo |
| A9 | A | Paleta das Definições no `nuon` (`set_palette`) + `scroll_to_visible` (pedidos do B) | feito |
| A10 | A | Núcleo das gravações: `recording/{recorder,compare,store}.rs` com testes | feito |
| A11 | A | Ligar a gravação ao ecrã de tocar + `ctx.last_result` + páginas `Results`/`Recordings` (encaminhamento) | feito |
| A12 | A | `widgets::bar_chart` e `widgets::line_chart` | feito |
| A13 | A | Notas "fantasma" no ecrã de tocar (comparar gravação com original) | por fazer |
| A14 | A | `midi_tools/meta.rs` (metadados + cache) e `midi_tools/edit.rs` (edição) com testes | por fazer |
| A15 | A | Cores do dropdown do `nuon` a partir da paleta | por fazer |
| A16 | A | Comparação por alinhamento (`align.rs`) + importação de pastas ligadas (`import.rs`) + `pretty_title` | feito |
| **B4** | B | Tradução PT-PT de `locals.rs` | feito |
| **B6** | B | Biblioteca nova (`library_page.rs`) | em revisão |
| **B5** | B | Menu principal novo (`main_page.rs`) | em revisão |
| **B1** | B | Definições (`settings.rs`) com tema e widgets | em revisão |
| **B2** | B | Faixas (`tracks.rs`) com tema e widgets | em revisão |
| **B3** | B | Barra de cima do ecrã de tocar (`top_bar/`) | em revisão |
| **B7** | B | Ecrã de resultado (`results_page.rs`) | por fazer |
| **B8** | B | Gravações de uma música (`recordings_page.rs`) + gráfico de evolução | por fazer |
| **B9** | B | Metadados na biblioteca: duração, dificuldade, capa gerada | bloqueada (A14) |
| **B10** | B | Página "Editar música" (`edit_page.rs`) | bloqueada (A14) |
| **B11** | B | Ecrã de estatísticas (`stats_page.rs`) | por fazer |

Ordem para o B: **B6 → B5 → B1 → B2 → B3**, e depois **B7 → B8 → B9 → B10 → B11** à medida que forem desbloqueadas.

---

## Especificações

### B4: tradução PT-PT
- Em `locals.rs`, acrescenta um ramo `"pt" => "…".into(),` a **todas** as funções.
- Passa os textos em português que estão escritos diretamente no código (por exemplo "Biblioteca", "+ Pasta", "Atualizar", "Limpar") para funções novas em `locals.rs`, numa secção `pub mod library`. O texto em inglês fica no ramo `_`.
- **Feito quando:** compila, e com o sistema em português nenhum texto aparece em inglês.

### B6: biblioteca (`library_page.rs`)
Problemas atuais, vistos na captura: o texto aparece centrado (o `nuon::label` centra por defeito, por isso usa `.text_justify(TextAlign::Start)` ou os widgets), o título "Playlist (0)" fica por cima dos botões e falta estrutura visual.

Layout (janela de 1080×720, que tem de funcionar até 670 px de largura):
```
┌ cabeçalho ───────────────────────────────────────────────────────┐
│ "Biblioteca" (FONT_XL, negrito)        [search_box 360px]        │
├ painel esquerdo (theme::panel, ~62%) ─┬ painel direito (~38%) ───┤
│ section_title("Músicas", Some(n))     │ section_title("Playlist",│
│ list_row por música:                  │   Some(n))  [Limpar]     │
│   título + subtítulo = nome da pasta  │ list_row com leading "1",│
│   trailing: [▶] [+] (icon_button 36)  │  active = a tocar,       │
│   selected = foco do teclado          │  trailing: ↑ ↓ ×         │
│ estado vazio: hint + botão primário   │ estado vazio: hint       │
│   "+ Adicionar pasta"                 │                          │
├ barra inferior ───────────────────────┴──────────────────────────┤
│ [←]  [pastas: chips com ×]                 [▶ Tocar playlist]    │
└──────────────────────────────────────────────────────────────────┘
```
- Usa **apenas** `widgets::*` e `theme::*`. O `neo_btn` não entra nesta página.
- Mantém o padrão `enum Act` (recolher e só depois aplicar).
- Teclado (em `library_page_window_event`): ↑/↓ mudam o cursor (o campo `lib_cursor` chega com a A7; até lá cria-o tu em `UiState` e avisa), Enter toca a música selecionada, Tab junta-a à playlist e as letras continuam a ir para a pesquisa.
- **Feito quando:** a captura `NEOTHESIA_PAGE=library` mostra o layout acima sem textos sobrepostos. Para testar com músicas, adiciona a pasta `~/Desktop/Projetos/MusicaApp/ios_app` (tem uns 25 `.mid`).

### B5: menu principal (`main_page.rs`)
```
        [logótipo, ~420px de largura, com a animação de entrada atual]
        "O teu piano, as tuas músicas"            (text_muted)

  ┌ card "Continuar" (largura 520) ───────────────────────────┐
  │ ▶  <nome da última música>              [Tocar]  (primary)│
  └───────────────────────────────────────────────────────────┘
  ┌ card ────┐ ┌ card ────┐ ┌ card ────┐ ┌ card ────┐
  │ Biblio-  │ │ Abrir    │ │ Tocar    │ │ Defini-  │   (4 em linha; 2×2 se
  │ teca  [L]│ │ fich. [O]│ │ livre [F]│ │ ções  [S]│    a largura < 900)
  └──────────┘ └──────────┘ └──────────┘ └──────────┘
  Recentes: até 5 list_row (depende de A7; até lá omite)
                                                    [Sair] (ghost, canto)
```
- Usa `widgets::card` com ícones de `crate::icons` e `widgets::kbd` para o atalho.
- Mantém a entrada em cascata (`ease_out`, `self.age`).
- O cartão "Continuar" só aparece se houver `self.state.song()` e mostra `song.file.name`.
- Os atalhos L/O no teclado vivem em `mod.rs`, que é meu. Diz-me quando precisares deles e eu ligo-os.
- **Feito quando:** a captura sem `NEOTHESIA_PAGE` mostra o layout acima.

### B1: definições (`settings.rs`)
- Troca as cores fixas por `theme::*`. O `nuon/src/settings.rs` é infraestrutura: se precisares de mudanças nele, descreve-as numa mensagem e eu faço-as.
- Cabeçalho "Definições" igual ao da biblioteca e o botão ← no mesmo sítio.
- **Feito quando:** a captura `NEOTHESIA_PAGE=settings` é coerente com a biblioteca.

### B2: faixas (`tracks.rs`)
- Cartões de faixa com `theme::panel`, nome do instrumento, número de notas e um seletor Auto / Humano / Mudo em 3 segmentos (`ghost`, e `primary` no ativo).
- **Feito quando:** a captura `NEOTHESIA_PAGE=tracks` (com `-- test.mid` no fim do comando, para abrir uma música) é coerente com o resto.

### B3: barra do ecrã de tocar (`top_bar/`)
- Mostra o título da música e, se houver, `ctx.library.position_label()` ("3 de 8").
- Botões ⏮ e ⏭ ativos segundo `has_previous()` / `has_next()`. Ao clicar, `previous()`/`advance()` e enviar `Play` (ver a mensagem do A de 23:25).
- **Feito quando:** com uma playlist de 2 ou mais músicas, ⏭ salta para a seguinte e ⏮ volta à anterior.

### B1 (atualização): paleta disponível
- `nuon::settings::set_palette(ui, Palette { … })` já existe (A9). Semeia-a no início de `settings_page_ui` com `theme::*`. Os campos estão em `nuon/src/settings.rs` (`surface`, `border`, `border_width`, `radius`, `label`, `row_text`, `row_muted`, `icon_tile`, `accent*`, `control*`, `separator`).
- A paleta fica guardada no `Ui` e não é global, por isso semeia-a no ecrã de definições (é barato).

### B6 (atualização): cursor visível
- Antes de construíres o `nuon::scroll("library_scroll")`, chama `nuon::scroll_to_visible(ui, "library_scroll", y_da_linha, ROW_H, altura_visível)` quando o cursor muda.

### B7: ecrã de resultado

Já existe uma **versão inicial funcional** feita pelo A (`results_page.rs`). Captura: `NEOTHESIA_PAGE=results ./target/release/neothesia test.mid`. O que falta:
- Traduzir os textos para `locals.rs`.
- Tooltip do gráfico por cima da barra e não como `hint` solto.
- Botão "Ver gravação" (abrir `take.midi_path`, como faz `recordings_page.rs`).
- Uma etiqueta de comparação com a gravação anterior ("+6 % do que a última vez"), usando `ctx.takes.list(&take.song)`.
- Botões e espaçamentos a funcionar a 670 px de largura.

Layout de referência:
```
┌──────────────────────────────────────────────────────────┐
│  <nome da música>                     <data · duração>   │
│                                                          │
│        87 %          142 certas   12 falhadas   5 a mais │
│     (grande, anima de 0 até ao valor em ~600 ms)         │
│  Ritmo: +23 ms (ligeiramente atrasado) · regularidade 41 ms │
│                                                          │
│  [ bar_chart: erros por compasso ]                       │
│                                                          │
│ [←]   [Ver gravação] [Gravações]  [Repetir] [▶ Seguinte] │
└──────────────────────────────────────────────────────────┘
```
- "Seguinte" só aparece se `ctx.library.has_next()`.

### B8: gravações de uma música
Já existe uma **versão inicial funcional** feita pelo A (`recordings_page.rs`). Captura: `NEOTHESIA_PAGE=recordings ./target/release/neothesia test.mid` (usa uma pasta temporária com 8 gravações de demonstração). O que falta:
- **Apagar com confirmação.** Não dá para desfazer, por isso o primeiro clique mostra "Confirmar?" no botão durante 3 s e só o segundo apaga.
- Entrar nesta página a partir da Biblioteca: um botão "Gravações" por música (`self.state.open_recordings(ctx, path)`), visível só se `ctx.takes.list(path)` não estiver vazio. Para não ler o disco em cada frame, pede-me um contador em cache se precisares.
- A data de cada ponto do gráfico como tooltip e um eixo com 0 % / 50 % / 100 %.
- Textos em `locals.rs`.

### B11: estatísticas
- Página nova `stats_page.rs`, com o encaminhamento pedido ao A (eu acrescento `Page::Stats`).
- Dados: `ctx.takes.list_all()` dá todas as gravações. Calcula tempo total tocado (soma das `duration`), número de gravações, músicas diferentes, melhor precisão por música (top 5) e dias seguidos com gravações (a partir de `recorded_at`).
- Cartões grandes com números animados (como a precisão em `results_page.rs`) e uma lista "Melhores músicas".

### Novidades para B6/B5/B8 (vindas da A16)
- **Títulos:** usa `Library::pretty_title(path)` → `(título, Option<compositor>)` nas linhas da biblioteca (título + subtítulo = compositor), no cartão "Continuar" e nos títulos das páginas.
- **B6, ligar uma pasta de gravações a uma música:** na linha de cada música, um menu ou botão "Ligar pasta de gravações…" (`rfd::AsyncFileDialog::pick_folder`, como em `pick_folder_fut`). Depois chama `ctx.library.link_recordings(pasta, música)` e `crate::recording::import::sync_folder(&ctx.takes, &música, &pasta)`. Se a música já tiver pasta (`ctx.library.linked_folder(&música)`), mostra o nome dela e a opção "Desligar".
- **B8:**
  - Seletor de métrica no gráfico de evolução: **Precisão** (`accuracy`), **Andamento** (`info.tempo_ratio`), **Regularidade** (`info.spread_ms`, menor é melhor; inverte o eixo ou mostra o valor) e **Hesitações** (`info.hesitations`).
  - Singular/plural ("1 hesitação").
  - Mostrar o trecho como "completa" quando cobre 90 % ou mais da música.
  - Botão "Sincronizar" que chama `sync_folder` e depois `reload_recordings`.
- **Utilizador real:** a pasta do Drive `…/Musicas Piano/Chopin` já está ligada à "Waltz in C♯ Minor" (7 gravações importadas). Captura com dados reais: `NEOTHESIA_PAGE=recordings NEOTHESIA_RECORDINGS_SONG="<caminho da valsa>" ./target/release/neothesia`.

---

# 🎨 REDESIGN "Sala de concerto" (aprovado pelo utilizador em 2026-11-20)

**Maqueta (referência visual obrigatória):** https://claude.ai/artifact/MbCKhD15iGuXLsTbTYWoqK
Quatro ecrãs: "Biblioteca por autor", "Página da música", "Comparação visual" e "Ecrã de tocar". Se não conseguires abrir o link, as regras abaixo chegam.

**O que o A já fez (no `feat/ui-redesign`, faz merge):**
- `theme.rs` com a paleta nova: fundo `#0F0E13`, superfícies, marfim `text()`, dourado `accent()` (a única cor de ação), `on_accent()` (texto escuro sobre o dourado), `good()` azul e `warn()` laranja (só em dados), e `FONT_DISPLAY` (serifa New York).
- Fontes: o texto normal passa a SF Pro sozinho. Para títulos usa `.font_family(theme::FONT_DISPLAY)`.
- `widgets::page_title`, `widgets::eyebrow` (rótulo em maiúsculas) e `section_title` (já em serifa). O botão `primary()` é dourado com texto escuro.
- Fundo dos menus calmo (sem riscas).
- `Library`: cada `LibSong` tem `title` limpo, `composer` e `composer_key`. `ctx.library.by_composer()` → `Vec<ComposerGroup { key, name, songs }>` (o grupo "Outros" fica no fim).
- Convenção de pastas: `<raiz>/<Autor>/<Música>/x.mid` dá autor e título a partir das pastas.
- `Page::Song` + `self.state.open_song(ctx, path)` + uma versão mínima de `song_page.rs`. Captura: `NEOTHESIA_PAGE=song NEOTHESIA_SONG="<caminho>"`.

**Regras visuais (todas as tarefas R):**
1. Fundo calmo. Painéis com `theme::surface()`, borda `border()`, raio 14–18. Nada de cores saturadas fora do dourado e dos dados.
2. **Um só botão dourado por ecrã** (a ação principal). O resto é `secondary`/`ghost`.
3. Títulos de página e nomes de músicas em destaque em `FONT_DISPLAY`. O resto em SF (por omissão).
4. Rótulos de secção com `widgets::eyebrow` ("A ESTUDAR", "AUTORES").
5. Os números importantes em grande (24–28 px, semibold) com a legenda pequena por baixo, em `text_muted`.
6. Usa sempre `title`/`composer` da `LibSong` ou `Library::pretty_title`. **Nunca mostres o nome do ficheiro.**
7. ⛔ Ecrã de tocar: só a `top_bar` (ver `ARQUITETURA.md` § 10).

| ID | Tarefa | Estado |
|---|---|---|
| **R1** | Navegação lateral partilhada (`menu_scene/shell.rs`, ficheiro novo, teu): logótipo "Neothasia" em serifa e itens Início, Biblioteca, Gravações, Estatísticas, Definições (item ativo com fundo dourado a 12 %). Largura 232 px; abaixo de 900 px de janela passa a só ícones (64 px). Bloco "Playlist" no fundo. | feito |
| **R2** | Biblioteca por autor (`library_page.rs`) | feito |
| **R3** | Página da música (`song_page.rs`) | feito |
| **R4** | Menu (`main_page.rs`) | feito |
| **R5** | Resultado (`results_page.rs`) | feito |

Ordem: **R1 → R2 → R3 → R4 → R5**. Faz captura de cada ecrã (uma de cada vez, com `NEOTHESIA_CAPTURE_FRAME=30`) e compara com a maqueta antes de passares a tarefa para `em revisão`.

**Atualização:** a A17 (análise) está feita. A R6 (ecrã Comparação, **incluindo o piano roll**) e a R7 (cascata) são do **B**. Ver a secção COMPARAÇÃO.

---

# 🔍 COMPARAÇÃO (pedido do utilizador: "quero perceber onde melhoro, de forma visual")

**Núcleo feito pelo A (A17), no `feat/ui-redesign`:** `recording/analysis.rs`. Lê `ARQUITETURA.md` § 11. O que importa:
`crate::recording::analysis::analyze_files(&música, &take.midi_path) -> Result<Analysis, String>`.
Demora cerca de 50 ms numa música inteira: **calcula só uma vez** quando a página abre e guarda o resultado num campo (`UiState` — pede-me o campo ou acrescenta `pub compare: Option<Analysis>` e avisa). **Nunca recalcules em cada frame.**

| ID | Tarefa | Estado |
|---|---|---|
| **R3a** | Correções da revisão de R1–R5 (lista na mensagem do A de hoje, "Revisão R1–R5") | feito |
| **R3b** | Regra "completa" (≥ 90 % do maior trecho gravado) + migalhas com o nome canónico do autor | feito |
| **R6** | Ecrã "Comparação" (`compare_page.rs`) | em revisão |
| **R6a** | Correções do ecrã Comparação (mensagem do A "R3b e R6 aceites") | por fazer |
| **R7** | **Comparação na cascata** (pedido explícito do utilizador). ⛔ **Sem mexer no desenho da cascata** (`ARQUITETURA.md` § 10). Abordagem obrigatória, a **faixa extra**: cria `neothesia/src/recording/cascade.rs` (autorizado, ficheiro novo) com `pub fn compare_song(song: &Path, take: &Take) -> Result<Song, String>`, que **(a)** carrega o `MidiFile` original; **(b)** corre `analyze_files`; **(c)** constrói uma `MidiTrack` nova com as notas de `analysis.played` (já no tempo do original) — `MidiNote` + os `MidiEvent` NoteOn/NoteOff correspondentes, `track_id` = nº de faixas, `track_color_id` diferente das outras; **(d)** devolve `Song` com essa faixa acrescentada, configurada como `PlayerConfig::Mute` (não soa) e visível. As faixas originais ficam como estão (soam). Botão "Ver na cascata" na R6 e em cada gravação: `connect_io` e `NeothesiaEvent::Play(song)`. **A gravação automática não pode gravar neste modo**: pede-me a flag (eu acrescento `ctx.compare_mode` ao `TakeSession`). | por fazer |

Ordem: termina primeiro a R1–R3 em que estiveres. Depois a **R6** e a seguir a **R7**. Antes de começares a R7, escreve em `MENSAGENS.md` como vais montar a `MidiTrack`, para eu validar.
