# Как запустить в Claude Code

## 0. Подготовка (один раз)
```bash
mkdir ibc-server && cd ibc-server && git init
# скопировать сюда содержимое этого архива: CLAUDE.md, docs/, .claude/agents/
git add -A && git commit -m "plan: contracts, agents"
# нужны: rustup (stable), node 22 LTS, pnpm 9+, just, docker; для мобилок — Xcode / Android Studio + NDK
claude   # запуск в корне репо; модель основной сессии — Opus
```
Проверить, что агенты подхватились: `/agents` — должно быть 14.

## 1. Промпт Фазы 0 (вставить целиком)
```
Ты оркестратор проекта ibc server. Прочитай CLAUDE.md, docs/PLAN.md (§0, §3, §6 Фаза 0) и docs/STATUS.md.
1) Сам выполни P0.1 (каркас). Убедись, что `pnpm tauri dev` и `cargo test --workspace` работают.
2) Запусти сабагента architect на P0.2 + P0.3.
3) Параллельно запусти release на P0.4.
4) Проверь гейт G0 (сборка, clippy, mock покрывает все команды ipc.md — проверь скриптом сравнения имён).
5) Обнови docs/STATUS.md, сделай коммит "phase 0". Мне — отчёт ≤ 15 строк.
Не читай код сабагентов целиком: только `git diff --stat` и вывод тестов.
```

## 2. Промпт Фазы 1 (после /clear)
```
Оркестратор ibc server. Прочитай CLAUDE.md, docs/STATUS.md, docs/PLAN.md §6 Фаза 1.
Запусти ПАРАЛЛЕЛЬНО, каждого в своём worktree: store (P1.STORE), vault (P1.VAULT), ssh-core (P1.SSH), flags (P1.FLAGS), ui-shell (P1.UI), qa (P1.QA).
vault зависит только от публичного API store — он уже есть со скелета Фазы 0.
После завершения: смёрджи ветки по одной, после каждой — `cargo clippy` + тесты затронутого крейта.
Затем `just e2e`. Затем security-reviewer на `git diff <phase0>..HEAD -- crates/vault crates/ssh`.
BLOCKER → верни задачу владельцу с текстом замечания. Обнови STATUS.md, коммит "phase 1".
```

## 3. Фаза 2
```
Оркестратор ibc server. CLAUDE.md, STATUS.md, PLAN.md §6 Фаза 2.
Параллельно: tauri-bridge (P2.BRIDGE), sftp-core (P2.SFTP), ui-terminal (P2.TERM), ui-sftp (P2.SFTPUI).
Когда sftp-core и tauri-bridge готовы — tauri-bridge подключает SFTP-команды (второй заход, SendMessage тому же агенту).
Затем qa (P2.QA) + бенчмарки, security-reviewer по src-tauri и crates/sftp. Проверь чеклист PLAN §1.1 вручную вместе со мной: дай список из 15 шагов для ручной проверки.
```

## 4. Фазы 3–5 — по тому же шаблону, задачи из PLAN.md §6.

## Экономия токенов — практические правила
- Каждая фаза — новая сессия (`/clear`), состояние только в STATUS.md.
- Если агент спросил про контракт — отвечает architect (SendMessage), а не оркестратор «на глаз».
- Не просите оркестратора «посмотреть код» — просите прогнать тесты/линтеры и показать diff --stat.
- Если `/cost` за фазу растёт выше ожидаемого — проверьте, не читают ли агенты `target/`, `node_modules/`, lockfiles (добавьте в `.claudeignore`/permissions deny).

## Рекомендуемый .claude/settings.json (permissions)
```json
{
  "permissions": {
    "deny": ["Read(./target/**)", "Read(./node_modules/**)", "Read(./ui/node_modules/**)",
             "Read(./**/Cargo.lock)", "Read(./**/pnpm-lock.yaml)", "Read(./ui/src/lib/api/bindings.ts)",
             "Bash(git push:*)", "Bash(rm -rf /:*)"],
    "allow": ["Bash(cargo:*)", "Bash(pnpm:*)", "Bash(just:*)", "Bash(git diff:*)", "Bash(git status:*)",
              "Bash(git log:*)", "Bash(docker compose:*)"]
  }
}
```
