"""Summarizes a macOS `sample` report: time by category and inclusive time by crate function."""
import collections
import re
import sys

text = open(sys.argv[1]).read()

# Flat profile: samples by the function at the top of the stack.
flat = text.split("Sort by top of stack, same collapsed (when >= 5):")[1]
categories = collections.Counter()
total = 0
for line in flat.splitlines():
    m = re.match(r"\s+(.*?)\s+\(in .*?\)\s+(\d+)$", line)
    if not m:
        continue
    name, n = m.group(1), int(m.group(2))
    total += n
    if re.search(r"malloc|free|realloc|memmove|bzero|memset|xzm_|raw_vec|finish_grow|DYLD-STUB\$\$(malloc|free|memcpy)|drop_glue|drop_slow", name):
        categories["allocation, freeing and copying"] += n
    elif re.search(r"sip|hash_one|Hasher|get_hash", name):
        categories["hashing"] += n
    elif re.search(r"fmt|format|write_str|replace|Formatter", name):
        categories["string formatting"] += n
    elif "memcmp" in name:
        categories["memory comparison"] += n
    else:
        categories["other (own code)"] += n
print(f"{total} samples by category:")
for name, n in categories.most_common():
    print(f"  {100 * n / total:5.1f}%  {name}")

# Inclusive profile from the call graph: count each function once per stack.
graph = text.split("Call graph:")[1].split("Total number in stack")[0]
inclusive = collections.Counter()
stack = []
for line in graph.splitlines():
    m = re.match(r"^(\s*[+!:| ]*)(\d+)\s+(.*?)\s+\(in ", line)
    if not m:
        continue
    depth, n, name = len(m.group(1)), int(m.group(2)), m.group(3)
    while stack and stack[-1][0] >= depth:
        stack.pop()
    if not any(s[1] == name for s in stack):
        inclusive[name] += n
    stack.append((depth, name))
top = max(inclusive.values())
print("\nInclusive time of crate functions (top 30):")
shown = 0
for name, n in inclusive.most_common():
    if "federation_composition_rs" not in name and "graphql" not in name:
        continue
    parts = re.findall(r"\d+([a-z_][a-z0-9_]+)", name)
    print(f"  {100 * n / top:5.1f}%  {'::'.join(parts[-4:])}")
    shown += 1
    if shown == 30:
        break
