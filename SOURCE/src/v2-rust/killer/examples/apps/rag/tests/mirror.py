"""Re-ranks the vectors dumped by `rag dump QUESTION... --index PATH` with plain Python cosine
similarity and checks that the Killer vec_search order is identical.

usage: python mirror.py DUMP.json     prints MATCH or the first difference
"""
import json
import math
import sys


def cos(a, b):
    na = math.sqrt(sum(x * x for x in a))
    nb = math.sqrt(sum(x * x for x in b))
    return sum(x * y for x, y in zip(a, b)) / (na * nb) if na and nb else 0.0


def main(path):
    with open(path, encoding="utf-8") as f:
        d = json.load(f)
    if not d["chunks"] or not d["queries"]:
        return "empty dump"
    for q in d["queries"]:
        ranked = sorted(((cos(q["vec"], c["vec"]), c["id"]) for c in d["chunks"]), key=lambda t: (-t[0], t[1]))
        want = [i for _, i in ranked[:5]]
        # f32 storage can swap chunks whose scores differ by ~1e-7: accept a different order only for such ties
        got = q["top"]
        if want != got:
            scores = {c["id"]: cos(q["vec"], c["vec"]) for c in d["chunks"]}
            for a, b in zip(want, got):
                if a != b and abs(scores[a] - scores[b]) > 1e-6:
                    return f"{q['q']!r}: python {want} vs killer {got}"
    return "MATCH"


if __name__ == "__main__":
    print(main(sys.argv[1]))
