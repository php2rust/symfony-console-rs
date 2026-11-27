# Symfony Console (subset)

A port of parts of `symfony/console` v8.1.9 (`630e7419`): first the parts
that the symfony-rs Yaml and Dotenv commands use, then what the Doctrine ORM
console commands need. MIT; Symfony source and tests copyright Fabien
Potencier. Unofficial, not affiliated with or endorsed by Symfony. The rest
of Console is not ported yet. "PLAN.md" and milestone names (M1-B, M7) below
refer to the symfony-rs plan, where this crate came from.

| Contract | Source | Evidence |
|---|---|---|
| Style tags (`<info>`, inline `fg`/`bg`/`href`/`options`, `</>`), escaping, nesting, decoration on and off, wrapping to a width by graphemes | `Formatter/OutputFormatter.php` | OutputFormatterTest (22 of 22) |
| Styles: colors, bright colors, hex colors by color mode, options, hyperlinks (not on JetBrains JediTerm, old Konsole or IDEA) | `Formatter/OutputFormatterStyle.php`, `Color.php`, `Output/AnsiColorMode.php` | OutputFormatterStyleTest (5 of 5) |
| Style stack: push, pop, pop to a matching style, nesting errors | `Formatter/OutputFormatterStyleStack.php` | OutputFormatterStyleStackTest (5 of 5) |
| Output: verbosity levels and per-message verbosity, `OUTPUT_NORMAL` / `RAW` / `PLAIN` (PHP's `strip_tags()` written from the manual), decoration, buffered and stream outputs, color support detection (`NO_COLOR`, `FORCE_COLOR`, TTY, `TERM`) | `Output/Output.php`, `Output/StreamOutput.php`, `Output/BufferedOutput.php` | OutputTest (11 of 11), StreamOutputTest (4 of 6, 2 `n/a`); `strip_tags()` on 6,137 calls recorded from PHP |
| Input: arguments and options (modes, defaults, shortcuts, negation, suggested values, deprecations), definitions and synopsis, ArrayInput and ArgvInput parsing (tokens from a list or `$_SERVER['argv']`), `getFirstArgument()`, `hasParameterOption()` / `getParameterOption()`, `getRawTokens()`, `__toString()`, `unparse()`; CompletionInput: tokens from a list or a string, what is being completed | `Input/*.php`, `Completion/CompletionInput.php`, `Completion/CompletionSuggestions.php`, `Completion/Suggestion.php` | InputArgumentTest (15 of 15), InputOptionTest (24 of 24), InputDefinitionTest (32 of 32), InputTest (11 of 12), ArrayInputTest (8 of 8), ArgvInputTest (23 of 23), CompletionInputTest (5 of 5) |
| Color mode from `COLORTERM` / `TERM`; width and height from `COLUMNS` / `LINES`, else `stty -a` | `Terminal.php` | Formatter tests; `stty` parsing unit test |
| `Helper::width()`, `length()`, `substr()`, `removeDecoration()` | `Helper/Helper.php` | Through the formatter and style tests |
| Commands: name, aliases and hidden flag from the name or `#[AsCommand]` (as a value), description, help and `getProcessedHelp()`, usages, synopsis, definition and its merge with the application's, `run()` (bind, `initialize()`, `interact()` when interactive, validate, code or `execute()`), `complete()`, helpers, code from a closure or an object's method with the method- and class-level attribute rules | `Command/Command.php`, `Command/InvokableCommand.php` (plain callables), `Attribute/AsCommand.php` | CommandTest (40 of 43, 3 `n/a`) |
| Helper sets and the formatter helper (`formatSection()`, `formatBlock()`, `truncate()`) | `Helper/HelperSet.php`, `Helper/HelperInterface.php`, `Helper/FormatterHelper.php` | HelperSetTest (5 of 5), FormatterHelperTest (9 of 9) |
| StringInput's tokens; NullOutput | `Input/StringInput.php`, `Output/NullOutput.php` | StringInputTest (3 of 3), NullOutputTest (10 of 11, 1 `n/a`); 3,015 `stripcslashes()` calls and 4,000 tokenized strings recorded from PHP |
| Application's registry: `addCommand()`, `register()`, `get()`, `has()`, `all()`, `find()` with abbreviations, case fallback, ambiguity and alternatives, `getNamespaces()`, `findNamespace()`, the default definition and helper set, the name and version, `setDefaultCommand()`, `complete()` | `Application.php` | ApplicationTest (45 of 147); 2,790 `find()` / `findNamespace()` calls and 8,000 `levenshtein()`, `strnatcasecmp()` and `<=>` calls recorded from PHP |
| Wrapping text to a width without cutting style tags or (by default) URLs | `Helper/OutputWrapper.php` | OutputWrapperTest (1 of 4, 3 `n/a`); 2,500 `wrap()` calls recorded from PHP |
| Tables: headers (one or several rows), rows, separators, TableCell colspan and rowspan, multi-line cells, cell styles and alignment, the seven styles and custom ones, column styles, minimum and maximum widths, header and footer titles, horizontal and vertical orientation | `Helper/Table.php`, `Helper/TableStyle.php`, `Helper/TableCell.php`, `Helper/TableCellStyle.php`, `Helper/TableSeparator.php`, `Helper/TableRows.php` | TableTest (32 of 36), TableStyleTest (1 of 1), TableCellStyleTest (1 of 1); 1,500 generated tables recorded from PHP |
| SymfonyStyle: blocks (success, error, warning, note, info, caution, comment, custom), outline blocks, title, section, listing, text, table, horizontal table, definition list, new lines and the blank lines it puts between them | `Style/SymfonyStyle.php`, `Style/OutputStyle.php`, `Output/TrimmedBufferOutput.php` | SymfonyStyleTest (6 of 22, 2 `n/a`) and 27 of its 28 output fixtures |
| GitHub Actions annotations | `CI/GithubActionReporter.php` | GithubActionReporterTest (2 of 2) |
| CommandTester's `execute()` and display; CommandCompletionTester | `Tester/CommandTester.php`, `Tester/TesterTrait.php`, `Tester/CommandCompletionTester.php` | Through CommandTest |

## Rust shape

- A subclass of Command is a `CommandHandler`: the methods PHP code
  overrides, its class name and its `#[AsCommand]` attribute as a value.
  Its methods receive the `Command` as `$this`. What a subclass does
  before `parent::__construct()` is `before_construct()`.
- A command's code is a `Code`: a closure, or an object's method with the
  attributes found on the method and its class.
- An application's commands are `SharedCommand`s (`find()` returns the
  object it holds). A command keeps an `ApplicationHandle` to what it reads
  from its application: the definition, the helper set and whether it
  runs a single command.

- PHP's environment is read through `env`: `getenv()` and `$_SERVER`, with
  `putenv()` and `$_SERVER` writes kept in a per-thread overlay (changing
  the process environment is `unsafe`). Terminal's static caches are per
  thread.
- Exceptions are `ConsoleError`, one variant per class, with upstream's
  messages.
- Like symfony-demo-rs's console, this behaves as Symfony does on Linux on
  every system (`stty -a` for the terminal size).

## Differences

- Not ported yet (M7): `Application::run()` and everything it drives
  (`doRun()`, `renderThrowable()`, `configureIO()`, events, signals,
  command loaders, the container), invokable commands' parameter
  attributes (`#[Argument]`, `#[Option]`, `#[MapInput]`, `#[Ask]`,
  `#[Interact]`), an invokable subclass of Command, `setInputs()` and
  `capture_stderr_separately` in the testers, and the help, list,
  `_complete` and `completion` commands' work: they are registered with
  their names, definitions and help, but running one is refused, and their
  suggestion closures are not ported.
- SymfonyStyle's questions, progress bars and trees, and ConsoleOutput's
  sections (which `appendRow()` and SymfonyStyle's tables use when the
  output has them) are M7 work; `appendRow()` refuses every output here.
- Table's style registry (`setStyleDefinition()`) is per thread.
- PHP binds an unbound closure given as code to the command (`$this`); a
  Rust closure captures what it uses instead (CommandTest's three binding
  tests are `n/a`).
- `setProcessTitle()` keeps the title (`get_process_title()`), but the
  process is not renamed: `cli_set_process_title()` has no safe Rust
  equivalent.
- A numeric command name (`10`) is an integer key in PHP's command array,
  so `getAlternatives()` returns it as an int; here it is a string. The
  messages are the same.
- NullOutput's formatter is an undecorated `OutputFormatter`, not a
  `NullOutputFormatter`.
- `HelperSet::set()` calls a helper's `setHelperSet()`, but the M1-B
  helpers keep nothing; `getHelperSet()` is not ported.
- `stripcslashes()` and the natural order are written from PHP's manual
  and recorded answers. Recording found two things the manual does not
  say: a final lone backslash is kept, and whitespace right after a run of
  digits is compared, not skipped.

- ArgvInput tokens are UTF-8 `String`s. A non-scalar token (upstream's
  "Argument values expected to be all scalars" row) cannot be written,
  and a process argument that is not UTF-8 is refused by
  `Input::argv_from_server()` where PHP would keep its bytes (PLAN.md,
  Decisions for the owner).

- The truecolor row of `testInlineStyleOptions` runs only when `COLORTERM`
  is `truecolor`, as upstream's does; it is skipped elsewhere.
- Graphemes, NFC and widths come from `symfony-string`; see its README.
