# ZUI network probe

Минимальный Cloudflare Worker для контролируемых протокольных проверок ZUI.

Endpoints:

- `/health` — healthcheck;
- `/json` — фиксированный JSON с маркером `zui-probe-v1`;
- `/redirect` — контролируемый редирект на `/json`;
- `/bytes` — небольшой бинарный ответ с поддержкой `Range`;
- `/ws` — WebSocket echo с ограничением сообщения до 4 КиБ;
- `/diagnostic` — фактические HTTP и TLS версии, которые увидел Worker.

При сборке с `ZUI_PROBE_BASE_URL` клиент отдельно проверяет JSON, redirect, Range,
WebSocket echo, HTTP/1.1, HTTP/2 и QUIC с ALPN `h3` на этом Worker.

Развёртывание:

```powershell
npm ci
npm run deploy
npm run healthcheck -- -BaseUrl https://zui-network-probe.<account>.workers.dev
```

Чтобы включить контролируемые проверки в сборку ZUI, задайте адрес Worker перед `tauri build`:

```powershell
$env:ZUI_PROBE_BASE_URL = "https://zui-network-probe.<account>.workers.dev"
npm run tauri build
```

Без этой переменной ZUI выполняет проверки Discord, YouTube и общих протоколов, но не добавляет диагностическую группу `ZUI Probe`.

Worker не записывает IP-адреса, заголовки, содержимое запросов или другие идентификаторы. В `wrangler.toml` отключена observability. Клиент ZUI не должен использовать доступность этого Worker как доказательство доступности Discord или YouTube: endpoint предназначен только для контролируемой диагностики HTTP, TLS, Range, WebSocket и QUIC/HTTP3.
