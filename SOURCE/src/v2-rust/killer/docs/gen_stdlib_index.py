#!/usr/bin/env python3
"""Regenerate the standard-library index in docs/STDLIB_INDEX.md.

Names come from `const BUILTINS` in src/compiler.rs (the real registry). Names listed in
parity/REGISTERED_BUT_DEAD.txt are flagged: they are registered but not callable.
Descriptions are one-liners written by hand below (DESC) or derived from the name prefix (PREFIX);
a name with no entry gets the generic text of its group. Run:  python docs/gen_stdlib_index.py
"""
import os
import re

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.dirname(HERE)

DESC = {
    # core / output
    "print": "write values separated by spaces, then a newline (same as println; use stdout_write for no newline)", "println": "write values separated by spaces, then a newline",
    "len": "length of a string, list, dict, set", "length": "alias of len",
    "push": "append an item to a list (in place)", "pop": "remove and return the last item of a list",
    "contains": "true if a list/string/dict contains the item", "index_of": "position of an item or substring, -1 if absent",
    "get": "dict lookup with a default: get(d, key, default)", "setdefault": "dict: insert default if key missing, return value",
    "copy": "shallow copy of a list or dict", "reversed": "reversed copy of a list or string",
    "range": "list of integers: range(n), range(a, b), range(a, b, step)", "type_of": "type name of a value",
    "type": "type name of a value: number, string, bool, null, array, dict, set, ...", "to_string": "convert to string",
    "str": "convert to string", "int": "convert to integer", "float": "convert to float", "bool": "convert to boolean",
    "to_int": "convert to integer", "to_float": "convert to float", "to_integer": "convert to integer",
    "keys": "list of dict keys", "values": "list of dict values", "entries": "list of [key, value] pairs of a dict",
    "sort": "sort a list in place or return sorted copy (see sorted)", "sorted": "sorted copy; optional key function",
    "reverse": "reverse a list or string (returns a new value)", "join": "join list items with a separator",
    "split": "split a string on a separator", "trim": "strip whitespace on both ends", "replace": "replace all occurrences of a substring",
    "upper": "uppercase a string", "lower": "lowercase a string", "starts_with": "string starts with prefix", "ends_with": "string ends with suffix",
    "startsWith": "alias of starts_with", "endsWith": "alias of ends_with", "substr": "substring by start and length",
    "substring": "substring by start and end", "slice": "slice(x, start, end) of a list or string",
    "format": "string formatting (registered but not implemented; use f-strings)", "parse_int": "parse text to int (registered but not implemented; use int)",
    "parse_float": "parse text to float (registered but not implemented; use float)", "parseInt": "parse text to integer", "parseFloat": "parse text to float",
    "abs": "absolute value", "floor": "round down", "ceil": "round up", "round": "round to nearest", "sqrt": "square root", "pow": "power",
    "sin": "sine", "cos": "cosine", "tan": "tangent", "random": "random float in [0, 1)",
    "bit_and": "bitwise and", "bit_or": "bitwise or", "bit_xor": "bitwise xor", "bit_shl": "shift left", "bit_shr": "shift right",
    "bit_not": "bitwise not", "bit_rotl": "rotate left", "bit_rotr": "rotate right",
    "min": "smallest of the arguments or of a list", "max": "largest of the arguments or of a list",
    "math_eval": "evaluate a math expression string", "math_eval_subst": "evaluate a math expression with variable substitution",
    "time_now": "current time", "time_ms": "milliseconds timestamp", "sleep": "pause for N milliseconds", "sleep_ms": "pause for N milliseconds",
    "concat": "concatenate lists or strings", "iterKeys": "iterate dict keys", "insert": "insert(list, index, item)",
    "delete": "delete(dict, key) or remove from a list", "set": "construct or mutate a set/collection", "entries ": "",
    "assert": "abort with an error if the condition is false", "assert_eq": "assert two values are equal",
    "assert_ne": "assert two values differ", "assert_true": "assert value is true", "assert_false": "assert value is false",
    "assert_contains": "assert a collection contains an item", "assert_nil": "assert value is null",
    "isNaN": "true if value is NaN", "isFinite": "true if value is a finite number", "String": "convert to string (JS-style alias)",
    "Number": "convert to number (JS-style alias)", "Boolean": "convert to boolean (JS-style alias)",
    "indexOf": "alias of index_of", "repeat": "repeat a string N times", "charAt": "character at index", "charCodeAt": "code of character at index",
    "includes": "alias of contains", "sum": "sum of a list", "enumerate": "list of [index, item] pairs", "all": "true if every item is truthy",
    "any": "true if some item is truthy", "zip": "pair up items of several lists",
    "readFile": "read a whole file as a string", "writeFile": "write a string to a file", "appendFile": "append a string to a file",
    "deleteFile": "delete a file", "fileExists": "true if the path exists", "fileSize": "size of a file in bytes", "fileCopy": "copy a file",
    "renameFile": "rename or move a file", "listDir": "list the names in a directory", "mkdir": "create a directory",
    "pathJoin": "join path parts", "pathBasename": "file name part of a path", "pathDirname": "directory part of a path",
    "pathExtension": "file extension of a path", "readBytes": "read a file as bytes", "writeBytes": "write bytes to a file",
    "system_time_ms": "wall-clock milliseconds since the Unix epoch", "thread_sleep_ms": "sleep the current thread",
    "parse_json": "parse JSON text", "json_parse": "parse JSON text into dicts/lists", "json_encode": "encode a value as JSON text",
    "json_stringify": "encode a value as compact JSON text", "json_pretty": "encode a value as indented JSON text",
    "parse_csv": "parse CSV text into rows", "to_csv": "encode rows as CSV", "to_yaml": "encode a value as YAML",
    "now": "current date-time value", "parse_datetime": "parse a date-time string", "format_datetime": "format a date-time",
    "date_now": "current date", "date_parse": "parse a date string", "date_format": "format a date", "date_add": "add days to a date",
    "date_diff": "difference between dates", "date_year": "year of a date", "date_month": "month of a date", "date_day": "day of a date",
    "timestamp": "seconds (or ms) since the Unix epoch as a number",
    "ord": "character code of a one-character string", "chr": "character for a code", "pad_left": "left-pad a string to a width",
    "pad_right": "right-pad a string to a width", "strip": "strip whitespace on both ends", "lstrip": "strip leading whitespace",
    "rstrip": "strip trailing whitespace", "capitalize": "capitalize the first letter", "title": "title-case a string",
    "swapcase": "swap letter case", "center": "center a string in a width", "zfill": "left-pad with zeros", "isdigit": "all characters are digits",
    "isalpha": "all characters are letters", "isalnum": "all characters are letters or digits", "isspace": "all characters are whitespace",
    "isupper": "all letters are uppercase", "islower": "all letters are lowercase", "splitlines": "split on line breaks",
    "find": "index of a substring, -1 if absent", "rfind": "last index of a substring, -1 if absent", "count": "count occurrences of a substring",
    "base64_encode": "Base64-encode", "base64_decode": "Base64-decode", "b64_encode": "Base64-encode", "b64_decode": "Base64-decode",
    "hex_encode": "hex-encode", "hex_decode": "hex-decode", "hmac_sha256": "HMAC-SHA256", "sha256": "SHA-256 digest (hex)",
    "sha256_bytes": "SHA-256 digest of bytes",
    "log": "natural logarithm", "log2": "base-2 logarithm", "log10": "base-10 logarithm", "exp": "e to the power x", "cbrt": "cube root",
    "trunc": "truncate toward zero", "sign": "sign of a number (-1, 0, 1)", "atan2": "two-argument arctangent", "hypot": "sqrt(x*x + y*y)",
    "asin": "arcsine", "acos": "arccosine", "atan": "arctangent", "sinh": "hyperbolic sine", "cosh": "hyperbolic cosine", "tanh": "hyperbolic tangent",
    "degrees": "radians to degrees", "radians": "degrees to radians", "gcd": "greatest common divisor", "lcm": "least common multiple",
    "mean": "arithmetic mean", "median": "median", "variance": "sample variance", "stdev": "sample standard deviation",
    "pvariance": "population variance", "pstdev": "population standard deviation",
    "env": "read an environment variable (null if unset)", "args": "list of command-line arguments", "cli_args": "list of command-line arguments",
    "exit": "exit the process with a status code", "process_exit": "exit the process", "env_get": "read an environment variable", "env_set": "set an environment variable",
    "gc_stats": "garbage-collector statistics", "jit_stats": "JIT statistics", "instanceof": "instanceof(obj, Class)",
    "dict_from_pairs": "build a dict from [key, value] pairs", "map": "map(list, fn): apply a function to every item",
    "filter": "filter(list, fn): keep items for which fn is truthy", "reduce": "reduce(list, fn, initial): fold a list",
    "hash_map_new": "create a hash map", "hash_map_insert": "insert into a hash map", "hash_map_get": "look up in a hash map",
    "hash_map_contains": "hash map contains key", "hash_map_remove": "remove from a hash map", "hash_map_size": "number of entries",
    "hash_map_keys": "keys of a hash map", "hash_map_values": "values of a hash map",
    "set_new": "new empty set", "set_from_array": "set from a list", "set_to_array": "list from a set", "set_add": "add an item to a set",
    "set_remove": "remove an item from a set", "set_has": "set contains item", "set_size": "number of items", "set_clear": "remove all items",
    "set_union": "union of two sets", "set_intersection": "intersection of two sets", "set_difference": "difference of two sets",
    "mutex_new": "create a mutex holding a value", "mutex_get": "read the mutex value", "mutex_set": "set the mutex value",
    "mutex_add": "atomically add to the mutex value", "mutex_free": "release a mutex",
    "chan_new": "create a channel", "chan_send": "send a value on a channel", "chan_recv": "receive from a channel (blocks)",
    "chan_try_recv": "receive without blocking", "chan_close": "close a channel",
    "async_spawn": "run a function asynchronously, returns a handle", "async_await": "wait for a handle and return its result",
    "async_all": "wait for all handles", "async_race": "first handle to finish", "async_sleep": "async sleep", "async_timeout": "await with a timeout",
    "async_chain": "chain async steps", "async_map": "map a function asynchronously", "async_resolve": "an already-resolved future",
    "async_reject": "an already-failed future", "await_future": "wait for a future", "spawn_thread": "run a function on an OS thread",
    "join_thread": "wait for a thread and get its result",
    "http_get": "HTTP GET", "http_post": "HTTP POST", "http_post_json": "HTTP POST a JSON body", "http_put": "HTTP PUT", "http_delete": "HTTP DELETE",
    "http_patch": "HTTP PATCH", "http_head": "HTTP HEAD", "http_status": "HTTP status code of a request", "http_download": "download a URL to a file",
    "http_get_json": "HTTP GET and parse JSON", "http_with_headers": "HTTP request with custom headers",
    "regex_match": "true if the pattern matches", "regex_find": "first match", "regex_find_all": "all matches", "regex_replace": "replace matches",
    "regex_split": "split on a pattern", "regex_test": "true if the pattern matches",
    "db_open": "open a key-value database file", "db_get": "read a key", "db_set": "write a key", "db_delete": "delete a key", "db_keys": "all keys",
    "db_keys_prefix": "keys with a prefix", "db_count": "number of keys", "db_close": "close the database", "db_drop": "delete the database",
    "readline": "read a line from stdin", "readline_prompt": "print a prompt, then read a line",
    "file_read": "read a file (registered but not implemented; use readFile)", "file_write": "write a file (registered but not implemented; use writeFile)",
    "file_exists": "file exists (registered but not implemented; use fileExists)", "file_delete": "delete a file (registered but not implemented; use deleteFile)",
    "file_append": "append to a file (registered but not implemented; use appendFile)",
    "T_POS": "trit constant: true", "T_NEG": "trit constant: false", "T_ZERO": "trit constant: unknown",
    "uncertain": "build an uncertain value: value with margin", "gauss": "uncertain value from mean and sigma",
    "unc_value": "central value of an uncertain number", "unc_margin": "margin of an uncertain number", "unc_lo": "lower bound", "unc_hi": "upper bound",
    "unc_sigma": "sigma of an uncertain number", "prob_gt": "probability that the value exceeds a threshold", "prob_lt": "probability that the value is below a threshold",
    "lint": "lint source text", "lint_code": "lint a source string", "lint_file": "lint a file", "fmt": "format source text", "fmt_file": "format a file",
    "help": "built-in help", "help_search": "search the help", "help_list": "list help topics",
    "call_native": "call a native function pointer", "sizeof": "size of a type in bytes", "alignof": "alignment of a type",
    "compress": "compress data", "decompress": "decompress data", "rle": "run-length encode", "lz77": "LZ77 compression (registered but not implemented)",
    "compress_ratio": "compression ratio", "compress_info": "compression information",
    "cpuid": "CPUID information (x86-64 only)", "rdtsc": "CPU timestamp counter (x86-64 only)",
    "auto": "reasoning-mode name (registered but not implemented as a function)", "factual": "reasoning-mode name (registered but not implemented as a function)",
    "math": "reasoning-mode name (registered but not implemented as a function)", "reasoning": "reasoning-mode name for the AI helpers",
    "web": "reasoning-mode name for the AI helpers", "rlm": "RLM reasoning entry point; needs a configured provider", "nova": "Nova format entry (registered but not implemented)",
    "auto_fix": "suggest an automatic fix for source or an error", "auto_test": "generate tests for source", "debug_check": "check source for likely bugs",
    "error_enhance": "add hints to an error message", "explain_error": "explain an error message", "killer_debug_agent": "AI debugging agent; needs a configured provider",
    "native_think": "local reasoning helper", "perf_profile": "profile a piece of code", "suggest": "suggest an improvement for source",
    "suggest_refactor": "suggest a refactoring for source", "import": "import a module at run time", "import_list": "list imported modules", "import_clear": "clear the import cache",
    "int_to_trit": "convert an integer to a trit", "to_bytes": "convert a value to bytes", "to_pointer": "convert an integer to a raw pointer (unsafe)",
    "fence": "memory fence (unsafe, low level)", "wfi": "wait-for-interrupt (unsafe, bare-metal only)",
    "errno": "last OS error number", "gdt_encode": "encode a GDT entry", "idt_encode": "encode an IDT entry",
}

# (prefix, group title, generic description)
PREFIX = [
    ("nova_", "Nova columnar data format", "Nova columnar-format helper"),
    ("kore_", "Kore data format", "Kore data-format helper"),
    ("trit_", "Trits (three-valued logic)", "trit operation"),
    ("tryte_", "Trits (three-valued logic)", "tryte (group of trits) operation"),
    ("fuzzy_", "Trits (three-valued logic)", "fuzzy-logic operation"),
    ("signal_", "Signals", "signal value with confidence"),
    ("qubit_", "Qubits", "qubit simulation operation"),
    ("ai_", "AI and LLM (needs a provider)", "AI helper; needs a configured provider"),
    ("llm_", "AI and LLM (needs a provider)", "LLM client call; needs a configured provider"),
    ("khlm_", "AI and LLM (needs a provider)", "KHLM assistant call"),
    ("kala_", "AI and LLM (needs a provider)", "Kala assistant call"),
    ("rlm_", "AI and LLM (needs a provider)", "RLM reasoning call"),
    ("ghost_", "AI and LLM (needs a provider)", "Ghost solver/assistant call"),
    ("vmem_", "AI and LLM (needs a provider)", "vector-memory store operation"),
    ("tool_", "AI and LLM (needs a provider)", "tool registry operation"),
    ("imagine", "AI and LLM (needs a provider)", "imagination module call"),
    ("affect_", "AI and LLM (needs a provider)", "affect-state call"),
    ("guardian_", "AI and LLM (needs a provider)", "guardian safety check"),
    ("image_", "AI and LLM (needs a provider)", "image helper"),
    ("ffi_", "Foreign function interface", "C FFI operation"),
    ("bytes_", "Bytes and memory", "byte-buffer operation"),
    ("ptr_", "Bytes and memory", "raw pointer operation"),
    ("mem_", "Bytes and memory", "raw memory access (unsafe)"),
    ("volatile_", "Bytes and memory", "volatile memory access (unsafe)"),
    ("io_port_", "Bytes and memory", "x86 I/O port access (unsafe, x86-64 only)"),
    ("mmap_", "Bytes and memory", "memory-mapped page operation (unsafe)"),
    ("atomic_", "Bytes and memory", "atomic memory operation"),
    ("int_to_bytes", "Bytes and memory", "integer to bytes"),
    ("bytes_to_int", "Bytes and memory", "bytes to integer"),
    ("pkg_", "Packages", "package-manager operation"),
    ("lsp_", "Tooling", "language-server operation"),
    ("dap_", "Tooling", "debug-adapter operation"),
    ("docs_", "Tooling", "documentation generator operation"),
    ("doc_", "Tooling", "doc-comment operation"),
    ("bench_", "Tooling", "benchmark helper"),
    ("repl_", "Tooling", "REPL completion helper"),
    ("stack_", "Tooling", "stack-trace helper"),
    ("watch", "Tooling", "value/file watcher"),
    ("ui_", "UI", "UI runtime helper"),
    ("mic_", "Device (Android only)", "microphone helper (registered but not implemented on desktop)"),
    ("call_detect_", "Device (Android only)", "call detection (registered but not implemented on desktop)"),
    ("phone_", "Device (Android only)", "phone helper (registered but not implemented on desktop)"),
    ("service_", "Device (Android only)", "Android service helper"),
    ("secure_", "Security", "encryption / secure-storage helper"),
    ("evidence_", "Security", "evidence hashing helper"),
    ("polyglot_", "Polyglot", "polyglot execution helper"),
    ("disk_", "OS-level (unsafe)", "raw disk access"),
    ("page_", "OS-level (unsafe)", "page-table operation"),
    ("interrupts_", "OS-level (unsafe)", "interrupt control (x86-64 only)"),
]

GROUPS = [
    "Core", "Strings", "Math and statistics", "Collections and sets", "Files, paths and time", "Data formats",
    "Concurrency", "Network", "Regex and database", "Trits (three-valued logic)", "Signals", "Qubits", "Uncertain values",
    "AI and LLM (needs a provider)", "Nova columnar data format", "Kore data format", "Foreign function interface",
    "Bytes and memory", "Packages", "Tooling", "UI", "Device (Android only)", "Security", "Polyglot", "OS-level (unsafe)", "Other",
]

NAMES_BY_GROUP = {
    "Strings": "upper lower trim strip lstrip rstrip split join replace starts_with ends_with startsWith endsWith substr substring repeat charAt charCodeAt "
               "ord chr pad_left pad_right capitalize title swapcase center zfill isdigit isalpha isalnum isspace isupper islower splitlines find rfind count "
               "format indexOf includes base64_encode base64_decode b64_encode b64_decode hex_encode hex_decode sha256 sha256_bytes hmac_sha256".split(),
    "Math and statistics": "abs floor ceil round sqrt pow sin cos tan asin acos atan atan2 sinh cosh tanh log log2 log10 exp cbrt trunc sign hypot degrees radians "
                           "gcd lcm mean median variance stdev pvariance pstdev random min max sum math_eval math_eval_subst bit_and bit_or bit_xor bit_shl bit_shr "
                           "bit_not bit_rotl bit_rotr isNaN isFinite parseInt parseFloat parse_int parse_float".split(),
    "Collections and sets": "len length push pop contains index_of get setdefault copy reversed range keys values entries sort sorted reverse insert delete concat "
                            "iterKeys slice enumerate zip all any map filter reduce dict_from_pairs set hash_map_new hash_map_insert hash_map_get hash_map_contains "
                            "hash_map_remove hash_map_size hash_map_keys hash_map_values set_new set_from_array set_to_array set_add set_remove set_has set_size "
                            "set_clear set_union set_intersection set_difference".split(),
    "Files, paths and time": "readFile writeFile appendFile deleteFile fileExists fileSize fileCopy renameFile listDir mkdir pathJoin pathBasename pathDirname "
                             "pathExtension readBytes writeBytes file_read file_write file_exists file_delete file_append readline readline_prompt time_now time_ms "
                             "sleep sleep_ms system_time_ms thread_sleep_ms now parse_datetime format_datetime date_now date_parse date_format date_add date_diff "
                             "date_year date_month date_day timestamp env env_get env_set args cli_args exit process_exit".split(),
    "Data formats": "parse_json json_parse json_encode json_stringify json_pretty parse_csv to_csv to_yaml compress decompress rle lz77 compress_ratio compress_info".split(),
    "Concurrency": "mutex_new mutex_get mutex_set mutex_add mutex_free chan_new chan_send chan_recv chan_try_recv chan_close async_spawn async_await async_all "
                   "async_race async_sleep async_timeout async_chain async_map async_resolve async_reject await_future spawn_thread join_thread".split(),
    "Network": "http_get http_post http_post_json http_put http_delete http_patch http_head http_status http_download http_get_json http_with_headers".split(),
    "Regex and database": "regex_match regex_find regex_find_all regex_replace regex_split regex_test db_open db_get db_set db_delete db_keys db_keys_prefix db_count db_close db_drop".split(),
    "Uncertain values": "uncertain gauss unc_value unc_margin unc_lo unc_hi unc_sigma prob_gt prob_lt".split(),
    "Core": "print println type type_of to_string to_int to_float to_integer str int float bool String Number Boolean assert assert_eq assert_ne assert_true "
            "assert_false assert_contains assert_nil instanceof gc_stats jit_stats help help_search help_list T_POS T_NEG T_ZERO".split(),
}


def group_of(name):
    for g, names in NAMES_BY_GROUP.items():
        if name in names:
            return g
    for pre, g, _ in PREFIX:
        if name.startswith(pre):
            return g
    return "Other"


def describe(name, dead):
    d = DESC.get(name)
    if d is None:
        for pre, g, text in PREFIX:
            if name.startswith(pre):
                d = text
                break
    if d is None:
        d = "builtin (no one-line description written yet)"
    if name in dead and "not implemented" not in d:
        d += " (registered but not implemented)"
    return d


def main():
    src = open(os.path.join(CRATE, "src", "compiler.rs"), encoding="utf-8").read().split("\n")
    i = next(k for k, l in enumerate(src) if "const BUILTINS" in l)
    blk = []
    i += 1
    while not src[i].strip().startswith("];"):
        blk.append(src[i])
        i += 1
    names = []
    for n in re.findall(r'"([A-Za-z_0-9]+)"', "\n".join(blk)):
        if n not in names:
            names.append(n)
    dead = set(open(os.path.join(CRATE, "parity", "REGISTERED_BUT_DEAD.txt"), encoding="utf-8").read().split())
    by = {g: [] for g in GROUPS}
    for n in names:
        by[group_of(n)].append(n)
    out = []
    out.append("# Killer standard library index\n")
    out.append("Generated by `docs/gen_stdlib_index.py` from `const BUILTINS` in `src/compiler.rs` "
               "(%d names). Do not edit by hand; regenerate. Names marked *registered but not implemented* "
               "appear in the registry but calling them fails (see `parity/REGISTERED_BUT_DEAD.txt`). "
               "Descriptions are one-liners, not full specifications: see `docs/LANGUAGE.md` for semantics and "
               "`parity/PARITY_REPORT.md` for what is verified.\n" % len(names))
    for g in GROUPS:
        if not by[g]:
            continue
        out.append("\n## %s\n" % g)
        out.append("| Name | Description |\n|------|-------------|")
        for n in sorted(by[g]):
            out.append("| `%s` | %s |" % (n, describe(n, dead)))
    with open(os.path.join(HERE, "STDLIB_INDEX.md"), "w", encoding="utf-8", newline="\n") as f:
        f.write("\n".join(out) + "\n")
    print("wrote STDLIB_INDEX.md: %d names" % len(names))


if __name__ == "__main__":
    main()
