import json
rows = []
for i in range(30000):
    rows.append({"id": i, "name": "user" + str(i), "ok": i % 2 == 0, "vals": [i, i + 1, i + 2]})
s = json.dumps(rows, separators=(",", ":"))
print(len(s))
back = json.loads(s)
print(len(back))
print(back[29999]["vals"][2])
s2 = json.dumps(back, separators=(",", ":"))
print(len(s2))
