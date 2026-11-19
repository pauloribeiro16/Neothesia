# Arquitetura do Neothasia

Documento mantido pelo **agente A (arquiteto)**. O agente B implementa seguindo estas regras. Para mudar uma regra, propõe a alteração em `.agents/MENSAGENS.md`.

## 1. Mapa de módulos

```
neothesia/src/
├── main.rs            ciclo de eventos winit, troca de cenas, render, captura
├── context.rs         Context: estado global partilhado entre cenas
├── library.rs         Library: pastas, músicas, playlist (persistência em library.txt)
├── theme.rs           cores, medidas, ease_out(), anim(), panel(), fit_text()
├── widgets.rs         componentes: button, icon_button, list_row, search_box, card,
│                      section_title, hint, kbd
├── song.rs            Song = MidiFile + configuração das faixas
├── locals.rs          textos traduzidos (en/pl/de, e pt a acrescentar)
├── icons.rs           glifos da fonte de ícones
├── utils/capture.rs   modo de captura PNG (desenvolvimento)
└── scene/
    ├── menu_scene/    menu: páginas Main, Library, Settings, TrackSelection, Exit
    │   ├── mod.rs           MenuScene, roteamento de páginas e teclado
    │   ├── state.rs         UiState (página atual, pesquisa, dispositivos)
    │   ├── library_page.rs  página Biblioteca
    │   ├── settings.rs      página Definições
    │   ├── tracks.rs        página Faixas
    │   └── neo_btn.rs       botão antigo (a substituir por widgets::button)
    ├── playing_scene/ ecrã de tocar (notas a cair, teclado, top_bar)
    └── freeplay/      tocar livre
```

Os crates irmãos (`nuon`, `neothesia-core`, `midi-file`, `wgpu-jumpstart` e outros) são infraestrutura. **Não os alteres** sem acordo do A.

## 2. Fluxo de dados

```
           NeothesiaEvent (via ctx.proxy)
 MenuScene ──Play(Song)──────────────► PlayingScene
     ▲                                     │
     └──────MainMenu(Option<Song>)─────────┘   (fim da música / Esc)
```

- **`Context`** sobrevive às trocas de cena e guarda `config` (definições), `library` (biblioteca e playlist), os dispositivos MIDI e o GPU.
- As cenas **não comunicam diretamente**: mudam de cena enviando um `NeothesiaEvent` por `ctx.proxy`.
- A playlist vive em `ctx.library` (`queue`, `queue_pos`). Para tocar a música seguinte, chama-se `ctx.library.advance()`, carrega-se o MIDI e envia-se `Play`.

## 3. UI: modo imediato (`nuon`)

A UI é redesenhada **todos os frames**. Cada função `*_page_ui` desenha a página e devolve as interações.

**Regras obrigatórias:**

1. **Recolher e só depois aplicar.** Durante o desenho não se altera `ctx.library` nem outro estado que esteja a ser iterado. Guarda-se a intenção num `enum Act` e aplica-se no fim da função. Exemplo: `library_page.rs`.
2. **IDs estáveis e únicos.** Usa `widgets::wid(("prefixo", índice))`. Dois widgets com o mesmo id partilham o estado de hover e de clique.
3. **Posicionamento** com `nuon::translate().x(..).y(..).build(ui, |ui| …)` (relativo e com âmbito) ou `.add_to_current(ui)` (avança o cursor).
4. **Listas compridas** dentro de `nuon::scroll("id").scissor_size(w, h)`.
5. **Sobreposições** (tooltips, dropdowns) em `nuon::layer().overlay(true)`.
6. **Nada de cores ou medidas soltas.** Tudo vem de `theme::*`. Se faltar um token, pede ao A para o acrescentar.
7. **Componentes primeiro.** Botões, linhas, pesquisa e cartões vêm de `widgets::*`. Se precisares de um componente novo usado em mais de um sítio, pede-o ao A em vez de o criar dentro de um ecrã.
8. **Animação** com `theme::anim(ui, id, alvo, velocidade)`, que guarda o estado por id no `Ui`. Durações curtas e `ease_out`.

### Layout de página (padrão)

```
┌────────────────────────────────────────────────────┐
│ cabeçalho: título (FONT_XL) + ações à direita      │  56 px
├────────────────────────────────────────────────────┤
│ conteúdo (painéis theme::panel, margem SPACE_XL)   │
│                                                    │
├────────────────────────────────────────────────────┤
│ barra inferior: ← voltar · ações · ação principal  │  60 px
└────────────────────────────────────────────────────┘
```

### Entrada de texto

O `nuon` não tem campos de texto. Cada página com pesquisa:
- guarda o texto em `UiState` (por exemplo `search`);
- trata o teclado em `MenuScene::window_event` para a sua `Page`: `text` acrescenta, Backspace apaga e Esc limpa ou volta;
- desenha com `widgets::search_box`.

### Teclado

Cada `Page` trata as suas teclas em `MenuScene::window_event`. Convenções: Esc volta ou limpa, Enter é a ação principal, ↑/↓ navegam e as letras são atalhos (só em páginas sem pesquisa).

## 4. Como acrescentar uma página

1. Acrescenta uma variante a `Page` em `state.rs`.
2. Cria `minha_page.rs` com `impl super::MenuScene { pub fn minha_page_ui(&mut self, ctx, ui) }`.
3. Encaminha-a em `MenuScene::main_ui` e no `match` de `window_event`.
4. Acrescenta a opção ao `NEOTHESIA_PAGE` em `MenuScene::new`, para a poderes capturar.

## 5. Persistência

| O quê | Onde | Formato |
|---|---|---|
| Definições | `settings.ron` (`neothesia-core/config`) | RON versionado. **Não acrescentes campos** sem o A, porque `deny_unknown_fields` parte as configs antigas |
| Biblioteca/playlist | `~/.config/neothesia/library.txt` | linhas `tipo<TAB>valor`. Tipos novos (por exemplo `fav`) são ignorados pelas versões antigas |

## 6. Qualidade

- `cargo check -p neothesia` sem erros antes de qualquer "feito".
- Captura do ecrã alterado para PNG (ver `utils/capture.rs`), com o caminho do PNG referido na mensagem de "feito".
- A lógica sem UI (como a `Library`) leva testes unitários (`#[cfg(test)]`).
- Commits pequenos e em português, terminando com a linha de co-autoria do agente.
