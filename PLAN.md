# i3-launcher-rs — Plano

Launcher estilo `dmenu`: uma **strip horizontal** (padrão no topo do monitor, posição
configurável), `override-redirect`, com **ícones dos aplicativos**, busca incremental e
execução de entrys `.desktop`.

```text
┌──────────────────────────────────────────────────────────────────────────┐
│ Run: firef▏   [🦊 Firefox]  [🌐 Chromium]  [📝 Gedit]  [⌨ Alacritty]      │
└──────────────────────────────────────────────────────────────────────────┘
```

## 1. Objetivo e escopo

| Dentro do escopo (v1)                              | Fora do escopo (roadmap)                    |
| -------------------------------------------------- | ------------------------------------------- |
| Strip topo/base, largura do monitor, margens, offset | Layout em grade (várias linhas)             |
| Ícones via especificação XDG Icon Theme            | Wayland / layer-shell                       |
| Busca substring + fuzzy, ordenação por score       | IME / composição de caracteres complexos    |
| Navegação por teclado + mouse (clique/scroll)      | Histórico/frequência de uso, `frecency`     |
| Execução de `.desktop` com `Exec` e `Terminal`     | Plugins, múltiplos providers                |
| Modo dmenu (stdin) para compatibilidade            | Temas carregados de arquivos externos       |
| Config TOML + flags estilo dmenu                   | Preview pane / ações por item               |

Alvo: **X11 + i3**. Sem runtime assíncrono, sem toolkit pesado: janela própria via
`x11rb`, desenho por software, inicialização < 50 ms com ~50 apps.

## 2. Decisões técnicas

| Área       | Escolha                  | Por quê |
| ---------- | ------------------------ | ------- |
| Janela     | `x11rb` (protocolo puro) | Controle exato de posição/tamanho, `override-redirect` (o i3 não gerencia, sem borda/gap), grab de teclado, multi-monitor via RandR 1.5 |
| Desenho    | `tiny-skia` (CPU)        | Rasterização antialiased, `Pixmap` compatível com `resvg`, sem GPU/compositor |
| Texto      | `cosmic-text`            | Shaping, fallback de fontes, rasterização de glifos (`SwashCache`) |
| Ícones     | `resvg` + `image` (png)  | SVG renderizado no tamanho final (nitidez), PNG redimensionado com Lanczos |
| Config     | `serde` + `toml`         | Schema tipado, `deny_unknown_fields` para pegar erros de digitação |
| Execução   | `std::process::Command`  | Sem `sh -c` para `.desktop` (respeita quoting do spec) |

Por que não GTK/egui/iced: uma janela gerenciada pelo WM recebe borda, gaps e perde
foco ao clicar fora; o launcher precisa se comportar como o dmenu (overlay efêmero,
grab exclusivo do teclado, sem decoração).

## 3. Arquitetura

```text
src/
  main.rs            parse de args → config → X11 → itens → loop de eventos
  lib.rs
  cli.rs             flags compatíveis com dmenu (-b -p -fn -nb -nf -sb -sf -m)
  config.rs          TOML, defaults, overrides de CLI, cores, dimensões
  app.rs             Item/Action: unifica entrys .desktop e linhas de stdin
  matching.rs        substring/fuzzy + score + filtro ordenado
  desktop/
    parser.rs        parser do Desktop Entry Spec (grupos, i18n, escapes)
    scan.rs          varredura XDG_DATA_DIRS + dedup + filtros (NoDisplay, OnlyShowIn...)
    launch.rs        Exec → argv (field codes), Terminal, detach
  icons/
    theme.rs         index.theme, herança, busca best-fit por tamanho/contexto
    mod.rs           IconCache: nome → Pixmap já escalado (PNG/SVG)
  x11/
    mod.rs           conexão, átomos, janela, grab, PutImage
    monitor.rs       RandR 1.5 → Xinerama → tela; monitor focado via _NET_ACTIVE_WINDOW
    keys.rs          keycode/keysyms → char + modificadores
  ui/
    state.rs         query, cursor, seleção, scroll, edição de linha
    layout.rs        geometria do prompt, input, viewport e células
    paint.rs         desenho no Pixmap (barra, texto, ícones, cursor, chevrons)
```

Fluxo de dados:

```text
.desktop ──scan──▶ Vec<Item> ──filtro(query)──▶ Vec<Match> ──layout──▶ Paint
      stdin ───────┘                                  ▲                  │
                                                     └── teclado/mouse ◀┘
Enter ──▶ Action::Launch(argv) | Action::Shell(cmd) | Action::Print(texto)
```

## 4. Posicionamento e geometria

Tudo derivado do monitor escolhido (`monitor.target`):

```text
monitor = RandR.monitors[i]                    (fallback: Xinerama, depois screen)
w  = config.bar.width  ?? monitor.width - 2*margin
x  = monitor.x + margin + align_offset         (left | center | right)
h  = config.bar.height
y  = monitor.y + offset                        (edge = top)
y  = monitor.y + monitor.height - h - offset    (edge = bottom)
```

Layout interno (uma linha, igual ao dmenu):

```text
[ padding ][ prompt ][ input: ~33% da largura ][ células de item ......... ]
                                                         └ viewport com scroll horizontal
célula = pad + ícone(icon_size) + gap + texto + pad     (altura = h - 2*padding)
```

- Seleção sempre visível: `offset` é ajustado em `state.rs` após cada movimento.
- Indicadores `<` / `>` quando há itens fora do viewport.
- Texto centralizado verticalmente pela baseline do `cosmic-text`; ícones
  alinhados ao centro da célula.

## 5. Ícones (XDG Icon Theme Spec)

Ordem de resolução para `Icon=<nome>`:

1. Caminho absoluto → usa direto.
2. Tema configurado (`appearance.icon_theme`), com herança (`Inherits`) recursiva.
3. `hicolor` (fallback padrão do spec).
4. `$XDG_DATA_DIRS/pixmaps`, `/usr/share/pixmaps` e `~/.icons`.
5. Ícone genérico (`application-x-executable`) ou placeholder desenhado.

Seleção de arquivo por diretório do tema (`index.theme`: `Size`, `Scale`, `Type`,
`Context`): prioriza `Context=Applications`, tamanho exato, depois maiores, depois
menores; `Type=Scalable` (SVG) renderizado sob demanda no tamanho alvo. Extensões:
`svg` → `png` (XPM fica de fora na v1).

Cache: `HashMap<(nome, tamanho), Option<Pixmap>>` — a resolução é lazy, só para itens
visíveis, e o resultado já sai no tamanho final.

## 6. Busca e ordenação

- Normalização `case folding` (config `case_sensitive = false`).
- `match_mode`:
  - `substring`: posição do match, bônus em início de palavra, penalidade por tamanho.
  - `fuzzy`: subsequência gulosa, bônus por caracteres consecutivos e início de palavra.
- Score desc, empate estável (ordem original). `sort = false` mantém ordem de scan/stdin.
- Campos pesquisáveis: `Name`, `GenericName`, `Keywords`, `Categories`.
- Highlight dos caracteres casados: previsto na estrutura (`positions`), habilitado na v2.

## 7. Teclado e edição (compat dmenu)

| Tecla                   | Ação |
| ----------------------- | ---- |
| `Esc`                   | sai (código 1) |
| `Enter`                 | executa o item selecionado |
| `↑` / `↓`, `Ctrl+p/n`   | move seleção |
| `←` / `→`               | move cursor de texto |
| `Home` / `End`          | início/fim do texto |
| `Backspace` / `Delete`  | apaga antes/depois do cursor |
| `Ctrl+W` / `Ctrl+U`     | apaga palavra / linha |
| `Tab`                   | completa com o item selecionado |
| `PgUp` / `PgDn`         | salta 10 itens |
| Clique / scroll         | seleciona / navega |

Modificadores lidos do `state` do evento X11; texto via `GetKeyboardMapping`
(níveis 0/1 + Shift). Sem IME — limitação documentada na v1.

## 8. Configuração (TOML)

Busca: `$I3L_CONFIG`, `--config`, `$XDG_CONFIG_HOME/i3-launcher/config.toml`,
`~/.config/i3-launcher/config.toml`. Ver `config.example.toml`. Exemplo:

```toml
[bar]
edge = "top"        # top | bottom
height = 44
margin = 0
offset = 0
width = "100%"      # px ou "%"
align = "left"      # left | center | right

[monitor]
target = "focused"  # focused | primary | pointer | 0 | "HDMI-A-0"

[appearance]
font = "DejaVu Sans"
font_size = 14.0
prompt = "Run: "
icon_theme = ""     # vazio = detecta (gtk settings / hicolor)
icon_size = 24
show_icons = true

[behavior]
match_mode = "fuzzy"
case_sensitive = false
sort = true
terminal = ""       # ex.: "alacritty -e"; vazio = $TERMINAL
print_selection = false
```

Flags CLI sobrescrevem o config: `-p/--prompt`, `-b/--bottom`, `-e/--edge`,
`-m/--monitor`, `-fn/--font`, `-nb/-nf/-sb/-sf`, `--icon-size`, `--no-icons`,
`--height`, `--dmenu`, `--print`, `-c/--config`.

## 9. Milestones

- [x] **M0 — Esqueleto**: Cargo, deps, config, CLI, docs, estrutura de módulos.
- [x] **M1 — Dados**: parser `.desktop` (i18n/escapes/field codes), scan XDG,
      resolução de ícones (PNG/SVG, herança), matching, testes unitários.
- [x] **M2 — Janela**: conexão X11, geometria por monitor (RandR/Xinerama),
      janela `override-redirect`, grab de teclado, Esc/Enter.
- [x] **M3 — UI**: pintura da strip (fundo, prompt, input, cursor, ícones),
      edição de texto, navegação, scroll horizontal, execução de apps.
- [x] **M4 — Polimento**: mouse, indicadores de scroll, detecção de tema GTK,
      cache de medição de texto, `--dmenu`, validação de `Exec`, benchmark.
- [x] **M5 — Entrega**: README com instalação/bind do i3, `make install`,
      `cargo clippy -D warnings` limpo, testes de integração, release build.

### Estado atual (medido nesta máquina: i3 4.22, Xorg, 1366x768, 38 apps)

```text
config:          0 ms
scan .desktop:   1-2 ms
janela X11:      0 ms (grab do teclado já ativo)
fontes + tema:   8-9 ms (fontconfig/Adwaita)
primeiro frame: 12-13 ms
TOTAL:          23-25 ms até a strip visível
```

Desvios do plano original:

- `bar.corner_radius` foi substituído por `appearance.sel_radius`: arredondar a
  janela exigiria visual ARGB + compositor, enquanto o destaque de seleção já
  fica bom em profundidade 24.
- `--print` foi adicionado para testes/scripts (imprime em vez de executar).
- Ferramentas de depuração em `examples/` (`measure`, `layout_dump`, `match_dump`).

### Próximos passos (v2)

1. Layout em grade (`layout = "grid"`, `rows = N`) para muitos apps.
2. `xkbcommon` para IME, teclas mortas e layouts não-latinos.
3. Histórico/frequência (`frecency`) e pinned apps.
4. Suporte a Wayland (camada `wlr-layer-shell`) mantendo o backend X11.
5. Preview/ações extras (abrir com terminal, copiar Exec, etc.).

## 10. Riscos e mitigações

| Risco | Mitigação |
| ----- | --------- |
| Grab de teclado não recebe eventos | Grab no `root` com `owner_events=true` + `SetInputFocus`; re-grab em `FocusIn` (igual dmenu) |
| Ícones lentos no primeiro frame | Cache por `(nome, tamanho)`; carregar só o viewport; pré-aquecer em thread se necessário |
| Fontes SVG ausentes (resvg sem `text`) | Ícones com `<text>` são raros; habilitar feature `text`/`system-fonts` se aparecer |
| Tema sem PNG/SVG no tamanho | Escalonamento Lanczos (PNG) / render vetorial (SVG) + fallback `hicolor`/pixmaps |
| HiDPI | `scale` no config (v1.1); hoje assume 1.0 e usa `icon_size`/`font_size` em px |
| Teclas mortas/não-latinas | Limitação v1; caminho futuro: `xkbcommon` para keymap completo |
| Tecla presa em outro cliente | Grab exclusivo no início; `XUngrabKeyboard` no cleanup (inclusive panic hook) |

## 11. Como testar

```bash
cargo test                 # parser, matching, resolução de ícones
cargo run --release        # abre a strip no monitor focado
cargo run --release -- --dmenu < /usr/share/applications/lista.txt
```

Bind sugerido no i3 (`~/.config/i3/config`):

```
bindsym $mod+d exec --no-startup-id i3-launcher
```
