# Termlite — план реализации

> Рабочее название. Аналог Termius / мобильных SSH-клиентов: SSH-терминал + SFTP как в Termius, папки серверов, флаги стран у папок и хостов (как у hosts в Remnawave). Мультиплатформа: macOS, Windows, Linux, iOS, Android.

## 0. Как работать с планом

- **Оркестратор** — основная сессия Claude Code (Opus). В Фазе 0 пишет каркас сам, дальше только раздаёт задачи сабагентам, проверяет гейты и обновляет `docs/STATUS.md`.
- **Исполнители** — сабагенты из `.claude/agents/`. Каждый владеет своими путями и не читает чужой код, только контракты.
- **Источник истины** — `docs/contracts/*.md` (EN). После Фазы 0 они замораживаются; изменения — только через ADR в `docs/adr/`.
- **Одна фаза = одна сессия.** Между фазами: `/clear`, затем «прочитай CLAUDE.md, docs/STATUS.md и раздел фазы N в PLAN.md».
- Независимые задачи одной фазы запускаются параллельно, каждая в своём git worktree (`isolation: worktree` у Agent tool / `claude --worktree`, если ваша версия Claude Code это поддерживает; иначе — последовательно).

## 1. Продукт

### 1.1 Что входит в MVP (desktop)
- Хосты и папки (вложенные), drag & drop, сортировка, поиск, теги.
- Флаг страны у папки и у хоста; хост наследует флаг ближайшей папки, если свой не задан.
- Наследование настроек от папки: user, port, identity, jump host, startup command, keepalive.
- Хранилище секретов (vault): пароли, ключи, passphrase — зашифрованы, разблокировка через keychain ОС или мастер-пароль.
- SSH-терминал: вкладки, авторизация паролем/ключом/зашифрованным ключом/keyboard-interactive/ssh-agent, known_hosts с TOFU, переподключение.
- SFTP как в Termius: две панели (локальная ↔ удалённая, удалённая ↔ удалённая), drag & drop, очередь передач, пауза/возобновление/докачка, рекурсивные папки, конфликты (заменить/пропустить/переименовать/докачать, «для всех»).
- Командная палитра (⌘K / Ctrl+K), тёмная и светлая темы, RU/EN.

### 1.2 После MVP
Фаза 3: проброс портов (L/R/SOCKS5), импорт `~/.ssh/config`, генерация и менеджер ключей, сплиты, сниппеты, зашифрованный экспорт/импорт, лёгкий мониторинг сервера (CPU/RAM/диск через exec, без агента на сервере), Touch ID / Windows Hello.
Фаза 4: iOS и Android.

### 1.3 Не делаем (non-goals)
Облачная синхронизация в MVP (позже — опционально, E2E, self-hosted), mosh, telnet/serial, AI-ассистент, командные аккаунты, встроенный редактор файлов (только «открыть во внешнем редакторе» с автозагрузкой изменений — Фаза 3, опционально).

## 2. Стек и обоснование

| Слой | Выбор | Почему | Отвергнуто |
|---|---|---|---|
| Оболочка | **Tauri 2** | Системный WebView, инсталлятор ~5–15 МБ, один код для desktop+mobile, Rust-ядро | Electron (Termius на нём: 100+ МБ, ~300 МБ RAM); Flutter (dartssh2 менее зрелый, xterm.dart слабее xterm.js) |
| SSH | **russh** | Чистый Rust, async, Apache-2.0, поддерживает chacha20-poly1305, aes-gcm, ed25519, strict-kex (защита от Terrapin) | libssh2 (C, хуже async, FFI на мобилках) |
| SFTP | **russh-sftp** | Работает поверх того же соединения | — |
| Терминал | **xterm.js 5 + WebGL** | Де-факто стандарт (VS Code), быстрый рендер, поиск, unicode | Свой рендер на canvas — месяцы работы |
| UI | **Svelte 5 + Tailwind 4 + bits-ui** | Минимальный рантайм, маленький бандл, быстрый | React (тяжелее), готовые UI-киты (раздувают бандл) |
| Хранилище | **SQLite (rusqlite bundled)** | Одна база на все платформы, транзакции, миграции | JSON-файлы (гонки, нет транзакций) |
| Крипто | **XChaCha20-Poly1305 + Argon2id** | Случайный 192-битный nonce безопасен, Argon2id — рекомендация OWASP | AES-GCM с 96-битным nonce (риск коллизий при random nonce) |
| Флаги | **circle-flags (SVG, MIT)** | Круглые миниатюры, минималистично | Эмодзи-флаги — **не отображаются на Windows** (показываются буквы) |
| Типы IPC | **tauri-specta** | TS-типы генерируются из Rust → меньше ошибок и токенов | Ручные типы |

## 3. Архитектура

```
┌──────────────────────── UI (Svelte, WebView) ────────────────────────┐
│ Sidebar(tree+flags) │ Tabs: Terminal(xterm) │ SFTP(2 panes) │ Editor  │
│ stores (runes) ── api/bindings.ts (generated) ── api/mock.ts (dev)    │
└───────────────▲──────────────────────────────────────┬────────────────┘
   invoke (JSON / raw bytes)                Channel<raw bytes | events>
┌───────────────┴──────────── src-tauri (glue) ─────────▼───────────────┐
│ commands/*  state(AppState)  session registry  transfer registry       │
│ резолвит креды из vault → передаёт в ssh/sftp; known_hosts из store    │
└──────┬──────────────┬───────────────┬──────────────┬──────────────────┘
   crates/store   crates/vault    crates/ssh     crates/sftp   (crates/geo opt.)
   SQLite         DEK/KEK,        russh client   transfers,
   tree+inherit   keychain        PTY, forwards  pipelining
```
- `ssh`/`sftp` ничего не знают о БД и vault: креды и known_hosts приходят через параметры/трейты. Это позволяет делать их параллельно и тестировать изолированно.
- Секреты живут только в Rust (`SecretString`/`SecretBox`, zeroize on drop). UI получает только ID и отпечатки.

## 4. Нефункциональные требования (цифры = критерии приёмки)

| Метрика | Цель |
|---|---|
| Инсталлятор | macOS dmg ≤ 15 МБ, Windows msi/nsis ≤ 12 МБ, AppImage ≤ 90 МБ (тянет WebKitGTK) |
| Холодный старт до интерактивного окна | ≤ 600 мс (M1/Ryzen 5, SSD) |
| RAM, 1 сессия, idle | macOS ≤ 120 МБ, Windows ≤ 200 МБ (WebView2 сам по себе тяжёлый), Linux ≤ 150 МБ |
| JS-бандл (gzip) | ≤ 450 КБ, включая xterm |
| Терминал | `cat` 100 МБ текста: UI не замирает > 100 мс, вывод прерывается Ctrl+C ≤ 300 мс |
| Задержка ввода (локальный эхо-цикл, LAN) | ≤ 20 мс сверх RTT |
| SFTP | ≥ 80% скорости OpenSSH `sftp` на 1 Гбит LAN и на канале RTT 100 мс |
| Список хостов | 5000 хостов, прокрутка 60 fps, поиск ≤ 30 мс |
| Подключение (без jump) | ≤ RTT×4 + 150 мс |

## 5. Безопасность (кратко, детали — `contracts/security.md`)
- Vault: случайный DEK (256 бит) шифрует каждый секрет XChaCha20-Poly1305 с AAD. DEK обёрнут ключом из keychain ОС и/или ключом из мастер-пароля (Argon2id m=64 МиБ, t=3, p=1).
- Автоблокировка по простою и при сне; DEK стирается из памяти (zeroize).
- SSH: строгая проверка ключа хоста, отключены SHA-1/DH-group1/ssh-rsa(SHA-1) по умолчанию, strict-kex обязателен.
- Защита от злого сервера: санитизация имён файлов SFTP (`..`, абсолютные пути, `CON`/`NUL`/`:` на Windows), OSC 52 (запись в буфер) выключен, OSC 8-ссылки — с подтверждением.
- WebView: строгий CSP, минимальные Tauri capabilities, без remote-контента, без shell-плагина.
- Supply chain: `cargo deny`, `cargo audit`, `pnpm audit`, lockfiles, SBOM (`cargo cyclonedx`), подписанные обновления (minisign).
- **Вне модели угроз (честно):** вредонос с правами пользователя на разблокированной машине может читать память процесса и keychain — от этого ни один SSH-клиент не защищает.

## 6. Фазы и задачи

Формат: `ID [агент] — что сделать. Вход → Выход. Приёмка.`

### Фаза 0 — Фундамент (последовательно)
- **P0.1 [orchestrator]** Каркас: `pnpm create tauri-app` (Svelte + TS) → перестроить в layout из CLAUDE.md; Cargo workspace, `rust-toolchain.toml`, pnpm workspace, `justfile`, `deny.toml`, clippy/rustfmt, eslint/prettier, `.editorconfig`, `.gitignore`.
  Приёмка: `pnpm tauri dev` открывает пустое окно; `cargo test --workspace` и `just lint` зелёные.
- **P0.2 [architect]** Довести контракты (`data-model.md`, `ipc.md`, `security.md`, `ui.md`) до точных Rust-типов DTO, проставить `Status: FROZEN v1`. ADR-001…005: Tauri, russh, крипто vault, стриминг IPC, флаги.
- **P0.3 [architect]** Скелеты крейтов с публичным API из контрактов (тела возвращают `Err(Error::Unimplemented)`, не `todo!()`), сгенерированные `bindings.ts` и **`ui/src/lib/api/mock.ts`** — полная in-memory реализация API с фейковыми хостами, папками, флагами и эхо-терминалом. Это позволяет UI-агентам работать параллельно с ядром.
- **P0.4 [release]** CI-скелет: lint + unit-тесты на ubuntu/macos/windows, кеш cargo/pnpm.
- **Гейт G0:** всё собирается на 3 ОС, контракты заморожены, mock API покрывает 100% команд из `ipc.md`.

### Фаза 1 — Ядро + UI-каркас (параллельно, 6 агентов)
- **P1.STORE [store]** Схема, миграции, CRUD, дерево папок, fractional sort keys для DnD, резолв наследования с указанием источника каждого поля, поиск (FTS5 по label/hostname/tags).
  Приёмка: тесты на циклы в дереве (запрет), перенос, удаление с `MoveToParent|DeleteContents`, наследование флага и настроек на 3 уровнях, 5000 хостов — `tree_list` ≤ 15 мс.
- **P1.VAULT [vault]** DEK/KEK, режимы `OsKeychain` / `MasterPassword` (+ опциональный keychain-кеш для биометрии), шифрование секретов, lock/unlock, смена пароля (перешифровка только обёртки DEK), формат бэкапа `.tlvault` (заголовок с версией).
  Приёмка: тесты KAT, неверный пароль → `WrongPassword` без утечки тайминга сверх Argon2, подмена AAD → ошибка расшифровки, Debug не печатает секреты, zeroize проверен.
- **P1.SSH [ssh-core]** Подключение, политика алгоритмов, авторизация (password, key, encrypted key, keyboard-interactive, agent: `SSH_AUTH_SOCK` / Windows pipe `\\.\pipe\openssh-ssh-agent` / Pageant), known_hosts через трейт (TOFU + блок при смене), PTY shell, resize, keepalive, jump host (цепочка до 3), credit-based backpressure.
  Приёмка: интеграционные тесты против docker OpenSSH (см. QA), смена ключа хоста блокирует подключение.
- **P1.FLAGS [flags]** Скрипт импорта circle-flags → `ui/static/flags/*.svg` (SVGO), реестр `flags.ts` (коды, алиасы, поиск по RU/EN через `Intl.DisplayNames`), компонент `<Flag code size>` с fallback (серый круг + код).
- **P1.UI [ui-shell]** Против `mock.ts`: layout, сайдбар-дерево (папки/хосты/флаги, DnD, клавиатурная навигация, виртуализация), редактор хоста/папки с плейсхолдерами наследования, flag picker, командная палитра, экран блокировки/онбординг vault, настройки, темы, i18n.
- **P1.QA [qa]** `tests/docker/compose.yml`: OpenSSH-серверы (password, key-only, keyboard-interactive, bastion + internal для jump, сервер с «злыми» именами файлов), фикстуры ключей (ed25519, rsa4096, ecdsa, зашифрованный ed25519), `just e2e`, CI-джоба.
- **Гейт G1:** все unit + интеграционные тесты SSH зелёные; **security-reviewer** прошёл по `vault` и `ssh` без блокеров.

### Фаза 2 — Интеграция MVP (desktop)
- **P2.BRIDGE [tauri-bridge]** Все команды из `ipc.md`, AppState, реестры сессий и передач, стриминг сырых байт (коалесинг до 64 КБ / 8 мс), события состояния, capabilities (минимум), CSP, замена mock на реальный API (флаг `VITE_API=mock|real`).
- **P2.TERM [ui-terminal]** xterm + WebGL (fallback на DOM при потере контекста), fit/resize с debounce, flow control через `shell_ack`, вкладки, поиск, copy/paste (bracketed paste, предупреждение при вставке многострочного текста), темы, диалоги host key / keyboard-interactive / passphrase, баннер переподключения.
- **P2.SFTP [sftp-core]** SFTP поверх существующего соединения, листинг/stat/mkdir/rename/rm -r/chmod/realpath, движок передач: очередь (N параллельных файлов, по умолчанию 3), pipelining запросов (64 × 32 КиБ, при `limits@openssh.com` — до max-read/write), докачка, рекурсия, remote↔remote через клиента, политика конфликтов, санитизация имён, сохранение mtime.
- **P2.SFTPUI [ui-sftp]** Две панели, хлебные крошки, виртуализированный список, сортировка, DnD между панелями и из ОС, контекстное меню, ящик передач (скорость, ETA, пауза/отмена/повтор), диалог конфликтов.
- **P2.QA [qa]** Сценарии: файл 1 ГБ (sha256 совпадает), докачка после обрыва, 10 000 файлов в папке, unicode-имена, path traversal, симлинк-петли; бенчмарки из §4 → `docs/bench.md`.
- **Гейт G2 = MVP:** чеклист §1.1 выполнен на macOS, Windows, Linux; цифры §4 сняты; security-reviewer по bridge + sftp.

### Фаза 3 — Функции уровня Termius
- **P3.1 [ssh-core → tauri-bridge → ui-shell]** Проброс портов: local, remote, dynamic SOCKS5 (CONNECT, без auth, bind 127.0.0.1 по умолчанию), автозапуск с хостом, индикатор в UI.
- **P3.2 [store + ssh-core]** Импорт `~/.ssh/config` (Host, HostName, User, Port, IdentityFile, ProxyJump, Include, wildcard-блоки → в папку «Imported») с предпросмотром; импорт ключей из `~/.ssh`.
- **P3.3 [vault + ui-shell]** Генерация ключей (ed25519 по умолчанию, RSA 4096), копирование публичного ключа, «установить ключ на сервер» (аналог ssh-copy-id через exec).
- **P3.4 [ui-terminal]** Сплиты (горизонт./вертик.), broadcast ввода в выбранные панели (с явным индикатором).
- **P3.5 [store + ui-shell]** Сниппеты (глобальные / для папки), вставка из палитры.
- **P3.6 [vault + ui-shell]** Экспорт/импорт зашифрованного бэкапа.
- **P3.7 [ssh-core + ui-shell] (опционально)** Мини-мониторинг: раз в 3 с exec `cat /proc/loadavg /proc/meminfo; df -P /` — без установки агента; только Linux-серверы.
- **P3.8 [tauri-bridge] (опционально)** Touch ID (LocalAuthentication) и Windows Hello для разблокировки.
- **Гейт G3:** интеграционные тесты для форвардов и импорта; security-review форвардов.

### Фаза 4 — Мобильные платформы
- **P4.1 [mobile]** `tauri android init`, `tauri ios init`, сборка и запуск на симуляторе/эмуляторе и реальном устройстве.
- **P4.2 [mobile]** Хранение ключа: iOS — keychain (`keyring`, если поддерживает iOS в выбранной версии; иначе свой Swift-плагин, `kSecAttrAccessibleWhenUnlockedThisDeviceOnly`), Android — свой Kotlin-плагин (Android Keystore, AES-GCM wrap DEK, `setUserAuthenticationRequired` для биометрии).
- **P4.3 [mobile + ui-terminal + ui-shell]** Адаптивный UI: список хостов → полноэкранный терминал; панель клавиш (Esc, Tab, Ctrl, Alt, ←↑↓→, `|`, `/`, `~`, `-`, сниппеты), свайп по панели = стрелки, pinch = размер шрифта; SFTP — одна панель + системный picker/share sheet.
- **P4.4 [mobile]** Жизненный цикл: Android foreground service на время активных сессий; iOS — сессии рвутся в фоне, поэтому быстрое авто-переподключение + опция «startup command: `tmux new -A -s main`».
- **P4.5 [mobile]** Биометрия (`tauri-plugin-biometric`), `FLAG_SECURE` на экранах vault (Android), размытие снимка при уходе в фон (iOS).
- **Гейт G4:** ручной чеклист на iPhone и Android (ввод через русскую/английскую раскладку, GBoard, вставка, поворот экрана, обрыв сети).

### Фаза 5 — Hardening и релиз
- **P5.1 [security-reviewer]** Полный аудит по чеклисту `security.md` §Checklist.
- **P5.2 [qa]** `cargo fuzz`: парсеры ssh_config, known_hosts, импорт ключей, заголовок `.tlvault`, санитайзер путей — по 30 мин на цель без крэшей.
- **P5.3 [qa]** Перф-проход по §4, профилирование горячих мест.
- **P5.4 [release]** Матрица сборок (tauri-action): macOS universal (подпись + нотаризация), Windows x64/arm64 (msi + nsis), Linux (AppImage, deb, rpm), Android (aab/apk), iOS (TestFlight). Апдейтер с подписью minisign, SBOM, changelog.
- **Гейт G5:** релиз-кандидат на всех платформах.

## 7. Распределение по агентам и экономия токенов

| Агент | Модель | Владеет | Фазы |
|---|---|---|---|
| architect | opus | `docs/contracts`, `docs/adr`, скелеты API | 0, арбитраж |
| store | sonnet | `crates/store` | 1, 3 |
| vault | sonnet | `crates/vault` | 1, 3 |
| ssh-core | sonnet | `crates/ssh` | 1, 3 |
| sftp-core | sonnet | `crates/sftp` | 2 |
| tauri-bridge | sonnet | `src-tauri` (кроме mobile-плагинов) | 2, 3 |
| ui-shell | sonnet | `ui/src` кроме terminal/sftp | 1–4 |
| ui-terminal | sonnet | `ui/src/lib/terminal`, `ui/src/routes/(app)/term` | 2–4 |
| ui-sftp | sonnet | `ui/src/lib/sftp`, `ui/src/routes/(app)/sftp` | 2, 4 |
| flags | haiku | `ui/static/flags`, `ui/src/lib/flags`, `scripts/flags` | 1 |
| mobile | sonnet | `src-tauri/gen`, `src-tauri/plugins/*-mobile` | 4 |
| qa | sonnet | `tests/`, `fuzz/`, `benches/` | 1–5 |
| security-reviewer | opus (read-only) | — | гейты |
| release | haiku | `.github/`, `scripts/release` | 0, 5 |

Принципы экономии:
1. **Контракты вперёд.** Агенты пишут против `contracts/*.md` и `mock.ts`, не читая чужую реализацию.
2. **Модель по сложности.** Opus — только архитектура и security-review; механика (флаги, CI) — Haiku; реализация — Sonnet.
3. **Узкий контекст.** В определении каждого агента — список «владею / могу читать / не трогаю».
4. **Короткие отчёты.** ≤ 150 слов, без кода. Оркестратор проверяет `git diff --stat` и тесты, а не читает код.
5. **Английский в машинных файлах.** CLAUDE.md, контракты и агенты на английском: кириллица токенизируется примерно в 1,5–2 раза дороже (уверенность ~80%, зависит от токенизатора). PLAN.md на русском читает только оркестратор.
6. **Одна фаза — одна сессия**, состояние переносится через `docs/STATUS.md`.
7. **Ревью диффом, не репозиторием:** security-reviewer получает `git diff <base>..HEAD -- <paths>`.

## 8. Риски и ограничения (прямо)
1. **Tauri на мобильных менее зрелый, чем на desktop.** Самое слабое место — ввод в xterm.js в мобильном WebView (композиция IME, автозамена, GBoard). Закладывайте отдельное время на P4.3; запасной вариант — нативное поле ввода поверх терминала, отправляющее байты.
2. **iOS убивает фоновые соединения** (~30 с после ухода в фон). Как у всех SSH-клиентов в App Store. Лечится только mosh (вне scope) или tmux на сервере.
3. **Android foreground service:** для Play нужен тип FGS. `dataSync` на Android 15 ограничен ~6 ч в сутки; `specialUse` требует обоснования при ревью (уверенность ~70% — проверить актуальные правила Play на момент релиза).
4. **russh ≠ OpenSSH.** Нет GSSAPI/Kerberos; ключи FIDO (`sk-ssh-ed25519`) скорее всего не поддерживаются (~75%); поддержку OpenSSH-сертификатов и agent forwarding проверить на выбранной версии (~60–65%). Постквантовый kex (`mlkem768x25519-sha256`) — включить, если есть в версии (~60%).
5. **Память WebView2 на Windows** — 100+ МБ на старте не зависят от нас.
6. **Подпись:** Apple Developer — $99/год (иначе Gatekeeper блокирует, на iOS без него не распространить). Windows без сертификата — предупреждение SmartScreen; EV/OV-сертификаты — от ~$200–600/год. Доступность Azure Trusted Signing для физлиц вне США/Канады — не уверен (~50%), проверить.
7. **Лицензии:** russh — Apache-2.0, xterm.js — MIT, circle-flags — MIT; DB-IP Lite (если включите автоопределение страны) — CC BY 4.0, нужна атрибуция в «О программе».
8. **Бренд:** не копировать название, иконки и ассеты Termius — только идеи UX.

## 9. Definition of Done (любая задача)
- Код в своих путях, `clippy -D warnings`/`pnpm check` чисты, тесты на новую логику есть и зелёные.
- Нет `unwrap` в прод-коде, нет секретов в логах/Debug.
- Изменения API отражены в контракте (через ADR) и в bindings.
- Отчёт ≤ 150 слов; `docs/STATUS.md` обновлён оркестратором.
