# <Sxx> — <screen>: specification

<!--
Template of a screen specification for tui-rtwk; the worked example is
s14/spec.md. Written from the screen's behaviour map (oracle rules with
file:line evidence); states what to build, in the port's terms.

- Short sentences, one meaning each; no "may", "usually", "etc.".
- API first: the Rust interface callers use, with a one-line comment per
  field; how it fits the app (intents with tickets, live data through
  providers, pending in the model) in at most five bullets.
- Use cases are the core: Given → actions → result, concrete values, one
  screen test each; *Accept* names the oracle test/snapshot or says `new`;
  map rows in brackets.
- Behaviour: the key table in precedence order, then numbered rules only for
  what the cases do not already say.
- Look: oracle snapshots as the reference; no geometry in the text.
- Not carried: one paragraph. Open points: questions for the person.
-->

**What it is.** <two or three sentences: purpose, who opens it, rtwk widgets>

Oracle: <files>; evidence: the behaviour map rows in brackets.

## 1. API

```rust
<the open/push function, the result type, the spec and row types>
```

How it fits the app:
- <result → intent with ticket; live data; pending state; nesting>

## 2. Use cases

**UC-01 <name>.** <Given>. <action> → <result>. <action> → <result>.
Accept: <oracle test / snapshot / new>. [<map rows>]

## 3. Behaviour

### Keys (keymap context `<ctx>`), in precedence order

| # | key | when | does |
|---|---|---|---|

### Rules

1. **<name>:** <rule>. [<map rows>]

## 4. How it looks

| state | oracle snapshot |
|---|---|

## 5. Not carried

<oracle mechanisms replaced by the API or done by rtwk, one paragraph>

## 6. Open points

- <question for the person>
