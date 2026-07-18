defmodule PytrexEngine.MixProject do
  use Mix.Project

  def project do
    [
      app: :pytrex_engine,
      version: "1.0.0",
      elixir: "~> 1.16",
      start_permanent: Mix.env() == :prod,
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {PytrexEngine.Application, []}
    ]
  end

  defp deps do
    [
      {:websockex, "~> 0.4"},
      {:jason, "~> 1.4"},
      {:msgpax, "~> 2.3"},
      {:plug_cowboy, "~> 2.0"}
    ]
  end
end
