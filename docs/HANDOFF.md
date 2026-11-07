# Передача работы (для Codex и любого следующего агента)

Дата: 2026-11-07. Писал Claude (Claude Code). Прочитай этот файл целиком перед первой правкой.
Дальше по порядку: `CLAUDE.md` (жёсткие правила проекта, они обязательны и для тебя), `docs/STATUS.md` (коротко о прогрессе), нужный раздел `docs/PLAN.md`, нужные разделы `docs/contracts/*.md`.

## 1. Что это за проект

**iBC Shell**: открытый кроссплатформенный SSH + SFTP клиент (Tauri 2 + Rust + SvelteKit/Svelte 5).
Ориентиры по уровню: Termius (функции) и SwiftServer (https://swiftserver.app: мониторинг серверов, оформление в духе Apple).
Владелец пишет по-русски, хочет «премиум, как Apple»: легко, лаконично, светлая тема по умолчанию.

- Репозиторий: `https://github.com/dramaformama/ibc-shell` (origin настроен, **ни разу не пушили: перед первым `git push` спросить владельца**).
- Папка: `/Users/georg/Documents/files/CodeINK/ibc-shell`, ветка `main`.
- Имена: крейты `ibc-shell-*`, пакет `ibc-shell`, идентификатор `app.ibcshell.desktop`, служба связки ключей `app.ibcshell`.

```
crates/store  SQLite, дерево папок/хостов, наследование настроек
crates/vault  шифрование секретов (XChaCha20-Poly1305 + Argon2id), связка ключей ОС, бэкапы
crates/ssh    russh-клиент: подключение, авторизация, ключи хостов, PTY
crates/sftp   SFTP и движок передач (очередь, докачка, конфликты)
crates/geo    опционально, IP → страна (выключено)
src-tauri     команды IPC, состояние, сессии, автоблокировка
ui            SvelteKit (static) + Svelte 5 runes + Tailwind 4 + bits-ui + xterm.js
docs          PLAN, STATUS, contracts (заморожены, менять только через ADR), adr, TESTING, security-review
```

## 2. Окружение и проверки

- Rust через brew rustup: `export PATH=/opt/homebrew/opt/rustup/bin:$PATH` (без этого `pnpm tauri dev` падает на `cargo metadata`).
- Запуск приложения: `pnpm tauri dev` из корня. Интерфейс стартует в режиме `vite --mode tauri` (файл `ui/.env.tauri` задаёт `VITE_API=real`). Без этого UI работает на имитации (`ui/src/lib/api/mock.ts`), а не на Rust-бэкенде. Однажды это уже сломало ручную проверку.
- Данные приложения: `~/Library/Application Support/app.ibcshell.desktop/ibc.db`. Старые копии `app.ibcserver.desktop` и `app.ibcshell.desktop.old-backup` можно не трогать.
- Гейт перед каждым коммитом в `main`:
  ```
  cargo clippy --workspace --all-targets -- -D warnings
  IBC_BIG_MIB=8 cargo test --workspace      # sftp e2e ~65 с
  cargo deny check
  pnpm -C ui check && pnpm -C ui test && pnpm -C ui lint
  ```
- После изменения IPC: `just bindings` перегенерирует `ui/src/lib/api/bindings.ts` и проверяет, что у мока есть каждая команда.
- Живые SSH-серверы для e2e: `just e2e` (Docker/colima, порты 2201–2205, `tester` / `test-pw-123`).
- Ручная проверка: `docs/TESTING.md`.

### Как смотреть UI без бэкенда (в браузере)
`cd ui && npx vite dev --port 5180` → http://127.0.0.1:5180, пароль мок-хранилища `ibc-shell`.
Скриншоты делались Playwright с уже скачанным Chromium:
`/Users/georg/Library/Caches/ms-playwright/chromium-1243/chrome-mac-arm64/Google Chrome for Testing.app/Contents/MacOS/Google Chrome for Testing` (передавать как `executablePath`).
Язык UI в моке выбирается в Настройках. Дерево хостов виртуализировано: искать строки по тексту, а не по индексу `treeitem`.

## 3. Что сделано (всё в `main`)

- Фазы 0–2 из PLAN: хранилище, vault, SSH, SFTP-движок и UI, мост IPC, усиление безопасности, ревью безопасности (`docs/security-review.md`, ADR-006/007).
- Переименование в iBC Shell, иконки и логотип (`docs/brand/`, `src-tauri/icons/`, `ui/static/`).
- Проверка обновлений через GitHub Releases: `ui/src/lib/util/update.ts`, ADR-008. CSP `connect-src` разрешает только `https://api.github.com`, ссылка открывается через `open_external`.
- **Новый интерфейс (несколько раундов по замечаниям владельца):**
  - Одна вкладка = один сервер. «Терминал» и «Файлы» — два вида внутри группы вкладок (`sessions.groups`, `groupKey`, `switchKind` в `ui/src/lib/stores/{sessions.svelte.ts,actions.ts}`).
  - Точка статуса у вкладки: зелёная — подключено, жёлтая мигает — подключается, красная — ошибка.
  - Крестик на вкладке, кнопка питания и ⌘W отключают сервер целиком. ⌘1…9 и Ctrl+Tab переключают серверы.
  - Кнопка боковой панели всегда первая в полосе вкладок (`TabBar.svelte`). Поиск стоит в верхней строке боковой панели рядом с кнопками окна.
  - Дубля «Открытые» в боковой панели нет: владелец явно не хочет повторять вкладки.
  - У хоста в дереве при наведении появляются кнопки «Терминал» и «Файлы» (`TreeRowContent.svelte`, стили в `tree.css`). Зелёная точка — сервер открыт.
  - Форма хоста (`HostEditor.svelte`): «Адрес + порт», «Название» (необязательно), «Флаг», блок «Вход» с переключателем «Пароль / Ключ SSH». Пароль сохраняется как identity типа `password` (создаётся или обновляется автоматически), ключ импортируется через `identity_import_file`. Прочее свёрнуто в «Дополнительно».
  - **Папки без учётных данных** (`FolderEditor.svelte`: только имя и флаг). Хосты раскладываются по папкам только перетаскиванием, выбора папки в форме нет.
  - SFTP: ширина колонок (Размер, Изменён, Права) и разделитель панелей тянутся мышью, двойной клик сбрасывает. Хранится в localStorage `ibc.sftp.layout` (`ui/src/lib/sftp/layout.svelte.ts`).
  - Подсветка вывода терминала в духе Termius (`ui/src/lib/terminal/highlight.ts` + тесты). Красит IP, URL, error/failed/denied, warning, ok/active/done, проценты и размеры. Трогает только текст со стандартным цветом на обычном экране, не трогает vim/top. Переключатель в Настройках, ключ `term.highlight`.
  - Оформление в стиле Apple (`ui/src/app.css`): системная палитра (`#f2f2f7` и белые карточки, тёмная тема `#1c1c1e`), кнопки-капсулы, круглые иконки-кнопки, класс `.card`, цвета разделов `--c-cyan/yellow/pink/purple/green/orange`, плавающая скруглённая панель редактирования (`Sheet.svelte`), **выбор акцентного цвета** (9 системных цветов Apple, `settings.accent`, атрибут `data-accent` на `<html>`, применяется до первой отрисовки в `app.html`).
  - Найденный и исправленный баг: терминал был `position:absolute` без относительного родителя и закрывал всё окно. Теперь у `.stage` и `.content` стоит `position: relative`.

## 4. Незавершённое (продолжать отсюда)

### 4.1 Метрики сервера в стиле SwiftServer (главная текущая задача)
Цель владельца: «забрать суть SwiftServer и оформить под Apple».
Нужны три вещи:
1. Живые CPU, нагрузка, память, диск, сеть и процессы **колонкой рядом с терминалом** (инспектор).
2. Главный экран **«Сводка»** с карточками подключённых серверов: кольца CPU и RAM, сеть ↑↓, ядра, RAM, диск, аптайм.
3. Цветные карточки разделов.

**Бэкенд** (делал отдельный агент, ветка `worktree-agent-a4fad7d4956fdd500`, worktree `.claude/worktrees/agent-a4fad7d4956fdd500`; если её уже слили в `main`, этот пункт закрыт):
- Команда `stats_sample(session_id) -> HostStats` выполняет один exec-канал на уже открытом SSH-соединении терминала.
- Скрипт читает `/proc/stat`, `/proc/net/dev`, `/proc/diskstats` дважды с `sleep 1`, затем `/proc/meminfo`, `loadavg`, `uptime`, `df -kP /`, `nproc`, `os-release`, `ps … --sort=-pcpu | head -9`, температуру.
- Парсинг в `crates/ssh/src/stats.rs`. Команда стоит в `PASSIVE_COMMANDS`, чтобы опрос не сбрасывал автоблокировку.
- Нужны ADR-009 и запись в `docs/contracts/ipc.md`, плюс мок в `ui/src/lib/api/mock`.
- Поля `HostStats`, от которых зависит UI: `cpu_pct, cores, cpu_cores[], cpu_user, cpu_system, cpu_nice, cpu_iowait, cpu_steal, load1, load5, load15, mem_total, mem_used, mem_cached, swap_total, swap_used, disk_total, disk_used, net_rx_bps, net_tx_bps, disk_read_bps, disk_write_bps, uptime_s, os_name?, temp_c?, procs[{pid,user,cpu,mem_bytes,command}]`.
- Проверь, что агент закончил: `git -C .claude/worktrees/agent-a4fad7d4956fdd500 log main..HEAD` и `git status`. Прогони весь гейт, затем слей, затем удали worktree и ветку.

**UI** (ветка `wip/stats-ui`, ещё не в `main`, потому что без бэкенда не проходит `pnpm check`):
- Готово:
  - `ui/src/lib/stores/stats.svelte.ts` — опрос раз в 2 с, пока есть наблюдатель, история 60 точек, ключ — session id.
  - `ui/src/lib/stats/` — `Ring.svelte` (кольцо), `Spark.svelte` (SVG-график), `CoreBars.svelte` (сегменты по ядрам), `format.ts` (+тест), `Inspector.svelte` (колонка 320 px с карточками CPU, нагрузки, памяти, сети, диска и процессов), `ServerCard.svelte` (карточка для «Сводки»).
  - `sessions.sessionIds` и `groupSession()`; `TerminalView` сообщает session id после подключения.
  - Строки i18n `stat_*`, `dash_*`, `unit_*` в обоих языках.
- Осталось:
  1. Слить бэкенд, затем `git merge wip/stats-ui`, подогнать имена полей и команды под реальный `bindings.ts` (`api.statsSample`, тип `HostStats`, `error.code === 'Unsupported' | 'NotFound'`).
  2. Встроить `Inspector` в `ui/src/routes/(app)/+page.svelte`. Сейчас терминалы монтируются в `.stage` абсолютным слоем; нужна обёртка: слева область терминала (`position: relative`), справа `<Inspector group={…}>`, когда активен вид «Терминал» и включён `ui.inspectorOpen`. Флаг хранить в localStorage. Кнопку (иконка `PanelRight` или `Gauge`) поставить в `TabBar.svelte` в блок `.tools` рядом с «Терминал / Файлы».
  3. «Сводка»: первая постоянная кнопка в полосе вкладок (иконка `LayoutGrid` или `Gauge`), активна при `sessions.activeId === null`. Экран — сетка `ServerCard` (`grid-template-columns: repeat(auto-fill, minmax(340px, 1fr))`), заголовок крупный, как в SwiftServer (`m.dash_title()`, 28–34 px, жирный), вверху плашки «Всего / В сети / Не в сети». Если серверов нет — нынешний приветственный экран.
  4. Скриншоты в светлой и тёмной теме в мок-режиме; `impeccable detect` по изменённым файлам; весь гейт; коммит.
- Дальше в духе SwiftServer (по желанию владельца, по одному): панель Docker (контейнеры: статус, образ, порты, CPU/RAM; через `docker ps`/`docker stats --no-stream` exec), карта мира с задержкой до серверов, пинг (RTT) в карточке, сниппеты, проброс портов (Фаза 3), Touch ID перед показом паролей.

### 4.2 Известные проблемы
- Вид «Файлы» открывает **отдельное** SSH-соединение (`SftpTarget::host`), отсюда второй вход и второй вопрос о ключе хоста. Лучше переиспользовать соединение терминала (`SftpTarget { kind: "session", session_id }`), если терминал этого сервера открыт.
- Старые хосты могут брать логин из настроек папки (наследование осталось в данных), а новая форма папки их больше не показывает. Нужно либо убрать наследование учётных данных, либо показать его.
- Открытые находки безопасности #6–#9 и #11–#14 в `docs/security-review.md`. Сначала #6 (нет защиты навигации и новых окон) и #12 (`core:event:default` слишком широкий; нужны `allow-listen` и `allow-unlisten`).
- Ручная приёмка G2 (`docs/TESTING.md`) владелец начал: хранилище создаётся, подключение и терминал работают на реальном сервере Ubuntu. SFTP, обрыв и докачка, автоблокировка вживую не пройдены.
- `FlagPicker` не виртуализирован (~270 строк). paraglide при сборке может тянуть плагин с CDN.
- Нет файла LICENSE (спросить владельца, как подписать). Нет скриншотов в README. Social preview для GitHub лежит в `docs/brand/`, загружается вручную после первого пуша.
- Нативные `<select>` в SFTP («Этот компьютер / сервер») выглядят системно, их стоит оформить.

## 5. Дальний план
`docs/PLAN.md`:
- **Фаза 3:** проброс портов L/R/SOCKS, импорт `~/.ssh/config`, менеджер и генерация ключей, сплиты, сниппеты, бэкапы, мониторинг (см. 4.1), Touch ID / Windows Hello.
- **Фаза 4:** iOS и Android на том же UI.
- **Фаза 5:** hardening, пайплайн релизов в GitHub Releases (подписи, обновления), README со ссылками на скачивание всех платформ.

## 6. Как работать с владельцем
- Отвечать по-русски, коротко и по делу. Он раздражается на медленных агентов и повторные «не завис?»: работай напрямую, коротко сообщай, что делаешь.
- Дизайн мерить уровнем Apple / SwiftServer. Чужие картинки, иконки и названия (Termius, SwiftServer) **не копировать**, брать только идеи и устройство.
- Сообщения коммитов — понятный русский журнал изменений («Подсветка вывода терминала, …»), а не `feat(x): …`.
- Новая функция = обновить README/README.en и `docs/TESTING.md`.
- Не пушить и не публиковать без спроса.
- Контракты `docs/contracts/*` заморожены: новая команда IPC = запись в `ipc.md` + ADR.
