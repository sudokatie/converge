defmodule Lattice.MixProject do
  use Mix.Project

  def project do
    [
      app: :lattice,
      version: "0.1.0",
      elixir: "~> 1.17",
      start_permanent: Mix.env() == :prod,
      escript: [main_module: Lattice.CLI.Main],
      deps: deps()
    ]
  end

  def application do
    [
      extra_applications: [:logger],
      mod: {Lattice.Application, []}
    ]
  end

  defp deps do
    [
      {:jason, "~> 1.4"},
      {:uuid, "~> 1.1"},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false},
      {:dialyxir, "~> 1.4", only: :dev, runtime: false}
    ]
  end
end
