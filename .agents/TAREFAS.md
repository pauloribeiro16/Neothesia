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
| A7 | A | Favoritos, recentes e `lib_cursor` na `Library`/`UiState` (para B5/B6) | por fazer |
| A8 | A | Rever e fazer merge do trabalho do B, reinstalar a app | contínuo |
| **B4** | B | Tradução PT-PT de `locals.rs` | em curso |
| **B6** | B | Biblioteca nova (`library_page.rs`) | por fazer |
| **B5** | B | Menu principal novo (`main_page.rs`) | por fazer |
| **B1** | B | Definições (`settings.rs`) com tema e widgets | por fazer |
| **B2** | B | Faixas (`tracks.rs`) com tema e widgets | por fazer |
| **B3** | B | Barra de cima do ecrã de tocar (`top_bar/`) | por fazer |

Ordem para o B: **B4 → B6 → B5 → B1 → B2 → B3**.

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
