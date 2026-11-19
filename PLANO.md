# Neothasia: plano geral

Fork do [Neothesia](https://github.com/PolyMeilex/Neothesia) em `github.com/pauloribeiro16/Neothesia`.
**Objetivo:** manter as animações de notas a cair, mas tornar a app bonita e fácil de usar no dia a dia: pôr músicas, organizá-las, escolhê-las e tocá-las seguidas.

## Princípios de UI

1. **Um ecrã, uma tarefa.** Menu para decidir, Biblioteca para escolher, ecrã de tocar para tocar.
2. **Tudo a 1 clique.** Tocar uma música, pô-la na playlist ou voltar à última música faz-se com um só clique.
3. **Teclado primeiro.** Pesquisa ao escrever, ↑/↓ para navegar, Enter para tocar, `+` para pôr na fila, Esc para voltar.
4. **Consistência.** Todas as cores, medidas e animações vêm de `neothesia/src/theme.rs`.
5. **Movimento com propósito.** As animações são curtas (150–500 ms, ease-out) e só servem para orientar: entrada, hover e seleção.
6. **Português de Portugal** em toda a interface.

## Linguagem visual

- Fundo animado escuro (o `BgPipeline` existente), com painéis semitransparentes por cima: `theme::panel`, cantos de 16 px e borda de 1 px a 8 % de branco.
- Acento roxo `#A051FF` para o estado ativo e azul `#3891FF` para hover e foco.
- Texto: claro `#F2F0F8`, secundário `#9692A8` e terciário `#646076`.
- Tipografia: 40 nos títulos, 28 nas secções, 20 nos destaques, 16 no corpo e 13 nos rótulos.
- Espaçamento em grelha de 4: 4 / 8 / 12 / 16 / 24 / 32.

## Fases

### Fase 0: base (A) ✅ em curso
- [x] Biblioteca + playlist com avanço automático (`library.rs`, `library_page.rs`)
- [x] Hover animado nos botões e entrada do menu em cascata
- [x] `theme.rs` com cores, medidas, `anim()` e `panel()`
- [ ] Modo de captura PNG (`utils/capture.rs`) para validar a UI sem abrir a janela
- [x] Coordenação por ficheiros em `.agents/`

### Fase 1: menu principal (A)
- Cabeçalho com o logótipo mais pequeno e um subtítulo.
- Cartão **"Continuar"** com a última música aberta e um botão grande ▶.
- Ações principais em cartões grandes com ícone: Biblioteca, Abrir ficheiro, Tocar livre, Definições.
- Lista **"Recentes"** com as últimas 5 músicas, cada uma a 1 clique.
- Atalhos visíveis: `L` biblioteca, `O` abrir, `F` livre, `S` definições.

### Fase 2: biblioteca (A)
- Layout em 3 zonas: **barra lateral** (Todas, Favoritas, Recentes e as pastas), **lista** central e **fila** à direita.
- Caixa de pesquisa visível, com ícone e botão para limpar.
- Linhas com título e pasta. Ao passar o rato aparecem os botões ▶ e +.
- Navegação por teclado (↑/↓/Enter/+) com a linha selecionada destacada.
- Ordenação por nome ou por adicionadas recentemente.
- Fila com a música a tocar destacada, reordenação, botões aleatório e repetir, e um total de músicas.

### Fase 3: definições e faixas (B)
- Redesenhar `settings.rs` com o tema: secções em `theme::panel`, títulos claros e controlos alinhados.
- Redesenhar `tracks.rs`: cartões de faixa com cor do instrumento, contagem de notas e escolha Auto/Humano/Mudo bem visível.

### Fase 4: ecrã de tocar (B)
- Na barra de cima: título da música e posição na playlist ("3 de 8").
- Botões ⏮ anterior e ⏭ seguinte quando há playlist, usando `ctx.library`.
- Barra de progresso mais legível.

### Fase 5: organização (A, com apoio de B)
- Favoritos (★) persistentes.
- Várias playlists com nome (criar, renomear, apagar).
- Duração e número de notas de cada música, guardados em cache.
- Aleatório e repetir (uma música / toda a lista).

### Fase 6: polimento
- Transições suaves entre páginas (deslizar).
- Tradução PT de todos os textos (`locals.rs`).
- Script `make install-app` que compila e instala `/Applications/Neothesia.app`.

## Divisão de trabalho

Ver `.agents/README.md` (regras e área de cada agente) e `.agents/TAREFAS.md` (quadro de tarefas).
