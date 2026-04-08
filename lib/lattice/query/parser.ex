defmodule Lattice.Query.Parser do
  @moduledoc """
  Parser for Lattice Query Language (LQL).

  A simple SQL-like query language for querying CRDT data.

  ## Syntax

      SELECT [fields] FROM collection [WHERE condition] [LIMIT n]

  ## Examples

      SELECT * FROM users
      SELECT name, email FROM users WHERE active = true
      SELECT * FROM orders WHERE total > 100 LIMIT 10
      SELECT count(*) FROM events WHERE type = 'click'

  ## Supported Operators

  - Comparison: =, !=, <, >, <=, >=
  - Logical: AND, OR, NOT
  - Functions: count(*), keys(*), values(*)
  """

  @type field :: String.t() | :all | {:function, atom(), [any()]}
  @type operator :: :eq | :neq | :lt | :gt | :lte | :gte
  @type condition :: {operator(), field(), any()} | {:and, condition(), condition()} | {:or, condition(), condition()} | {:not, condition()}
  @type query :: %{
          select: [field()],
          from: String.t(),
          where: condition() | nil,
          limit: integer() | nil
        }

  @doc """
  Parse a query string into a query structure.
  """
  def parse(query_string) when is_binary(query_string) do
    query_string
    |> String.trim()
    |> tokenize()
    |> parse_tokens()
  end

  @doc """
  Tokenize a query string.
  """
  def tokenize(query_string) do
    # First, extract quoted strings and replace with placeholders
    {processed, quotes} = extract_quoted_strings(query_string)

    # Tokenize the processed string - order matters!
    # Replace multi-char operators first
    tokens =
      processed
      |> String.replace("<=", " <= ")
      |> String.replace(">=", " >= ")
      |> String.replace("!=", " != ")
      |> String.replace(~r/([(),])/, " \\1 ")
      # Now replace single-char operators, but not if part of multi-char
      |> String.replace(~r/(?<![<>!])=(?![<>])/, " = ")
      |> String.replace(~r/(?![=])<(?![=])/, " < ")
      |> String.replace(~r/(?![=])>(?![=])/, " > ")
      |> String.split(~r/\s+/, trim: true)

    # Restore quoted strings and normalize
    tokens
    |> restore_quotes(quotes)
    |> Enum.map(&normalize_token/1)
  end

  defp extract_quoted_strings(str) do
    extract_quoted_strings(str, "", %{}, 0)
  end

  defp extract_quoted_strings("", result, quotes, _n), do: {result, quotes}

  defp extract_quoted_strings(<<"'", rest::binary>>, result, quotes, n) do
    case String.split(rest, "'", parts: 2) do
      [quoted, remaining] ->
        placeholder = "__Q#{n}__"
        extract_quoted_strings(remaining, result <> placeholder, Map.put(quotes, placeholder, quoted), n + 1)

      [_] ->
        {result <> "'" <> rest, quotes}
    end
  end

  defp extract_quoted_strings(<<"\"", rest::binary>>, result, quotes, n) do
    case String.split(rest, "\"", parts: 2) do
      [quoted, remaining] ->
        placeholder = "__Q#{n}__"
        extract_quoted_strings(remaining, result <> placeholder, Map.put(quotes, placeholder, quoted), n + 1)

      [_] ->
        {result <> "\"" <> rest, quotes}
    end
  end

  defp extract_quoted_strings(<<c, rest::binary>>, result, quotes, n) do
    extract_quoted_strings(rest, result <> <<c>>, quotes, n)
  end

  defp restore_quotes(tokens, quotes) do
    Enum.map(tokens, fn token ->
      case Map.get(quotes, token) do
        nil -> token
        quoted -> "'" <> quoted <> "'"
      end
    end)
  end

  defp normalize_token(token) do
    case String.upcase(token) do
      "SELECT" -> :select
      "FROM" -> :from
      "WHERE" -> :where
      "LIMIT" -> :limit
      "AND" -> :and
      "OR" -> :or
      "NOT" -> :not
      "TRUE" -> true
      "FALSE" -> false
      "*" -> :all
      "=" -> :eq
      "!=" -> :neq
      "<" -> :lt
      ">" -> :gt
      "<=" -> :lte
      ">=" -> :gte
      "(" -> :lparen
      ")" -> :rparen
      "," -> :comma
      _ -> parse_value(token)
    end
  end

  defp parse_value(token) do
    cond do
      # Quoted string
      String.starts_with?(token, "'") and String.ends_with?(token, "'") ->
        String.slice(token, 1..-2//1)

      String.starts_with?(token, "\"") and String.ends_with?(token, "\"") ->
        String.slice(token, 1..-2//1)

      # Number
      String.match?(token, ~r/^-?\d+$/) ->
        String.to_integer(token)

      String.match?(token, ~r/^-?\d+\.\d+$/) ->
        String.to_float(token)

      # Identifier
      true ->
        String.downcase(token)
    end
  end

  defp parse_tokens(tokens) do
    with {:ok, select, rest} <- parse_select(tokens),
         {:ok, from, rest} <- parse_from(rest),
         {:ok, where, rest} <- parse_where(rest),
         {:ok, limit, _rest} <- parse_limit(rest) do
      {:ok,
       %{
         select: select,
         from: from,
         where: where,
         limit: limit
       }}
    end
  end

  defp parse_select([:select | rest]) do
    {fields, remaining} = parse_fields(rest)
    {:ok, fields, remaining}
  end

  defp parse_select(_), do: {:error, "Expected SELECT"}

  defp parse_fields(tokens) do
    parse_fields(tokens, [])
  end

  defp parse_fields([:from | _] = tokens, acc), do: {Enum.reverse(acc), tokens}
  defp parse_fields([], acc), do: {Enum.reverse(acc), []}

  defp parse_fields([:all | rest], acc) do
    case rest do
      [:comma | more] -> parse_fields(more, [:all | acc])
      _ -> parse_fields(rest, [:all | acc])
    end
  end

  defp parse_fields([field, :lparen, :all, :rparen | rest], acc)
       when field in ["count", "keys", "values"] do
    func = {:function, String.to_atom(field), [:all]}

    case rest do
      [:comma | more] -> parse_fields(more, [func | acc])
      _ -> parse_fields(rest, [func | acc])
    end
  end

  defp parse_fields([field | rest], acc) when is_binary(field) do
    case rest do
      [:comma | more] -> parse_fields(more, [field | acc])
      _ -> parse_fields(rest, [field | acc])
    end
  end

  defp parse_fields(tokens, acc), do: {Enum.reverse(acc), tokens}

  defp parse_from([:from, collection | rest]) when is_binary(collection) do
    {:ok, collection, rest}
  end

  defp parse_from([:from | _]), do: {:error, "Expected collection name after FROM"}
  defp parse_from(_), do: {:error, "Expected FROM clause"}

  defp parse_where([:where | rest]) do
    {condition, remaining} = parse_condition(rest)
    {:ok, condition, remaining}
  end

  defp parse_where(tokens), do: {:ok, nil, tokens}

  defp parse_condition(tokens) do
    parse_or_expr(tokens)
  end

  defp parse_or_expr(tokens) do
    {left, rest} = parse_and_expr(tokens)

    case rest do
      [:or | more] ->
        {right, final} = parse_or_expr(more)
        {{:or, left, right}, final}

      _ ->
        {left, rest}
    end
  end

  defp parse_and_expr(tokens) do
    {left, rest} = parse_comparison(tokens)

    case rest do
      [:and | more] ->
        {right, final} = parse_and_expr(more)
        {{:and, left, right}, final}

      _ ->
        {left, rest}
    end
  end

  defp parse_comparison([:not | rest]) do
    {expr, remaining} = parse_comparison(rest)
    {{:not, expr}, remaining}
  end

  defp parse_comparison([:lparen | rest]) do
    {expr, remaining} = parse_condition(rest)

    case remaining do
      [:rparen | final] -> {expr, final}
      _ -> {expr, remaining}
    end
  end

  defp parse_comparison([field, op, value | rest])
       when op in [:eq, :neq, :lt, :gt, :lte, :gte] do
    {{op, field, value}, rest}
  end

  defp parse_comparison(tokens), do: {nil, tokens}

  defp parse_limit([:limit, n | rest]) when is_integer(n) do
    {:ok, n, rest}
  end

  defp parse_limit(tokens), do: {:ok, nil, tokens}
end
