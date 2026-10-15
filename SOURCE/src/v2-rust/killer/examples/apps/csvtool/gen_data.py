# Deterministic sample data for the csvtool example (python gen_data.py).
import csv, random

random.seed(7)
regions = ["North", "South", "East", "West", "Zürich", "São Paulo"]
products = ["Widget", "Gadget, large", 'Gizmo "Pro"', "Doohickey"]
with open("data/sales.csv", "w", newline="", encoding="utf-8") as f:
    w = csv.writer(f, lineterminator="\n")
    w.writerow(["id", "region", "product", "units", "amount"])
    for i in range(1, 241):
        w.writerow([i, random.choice(regions), random.choice(products), random.randint(1, 40), "%.2f" % (random.randint(100, 99999) / 100)])
