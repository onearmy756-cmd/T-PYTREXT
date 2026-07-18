import Config

config :pytrex_engine,
  websocket_port: 42351,
  cluster_cookie: :pytrex_secret

config :logger, :console,
  level: :info,
  format: "$time $metadata[$level] $message\n",
  metadata: [:request_id]
