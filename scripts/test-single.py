# requires modifier importer/src/calculations to eprint results as json

import os
import subprocess
import json

SOURCE = "D:/data/"
OUTPUT = "D:/projects/treee/cwd/auto/single"

TYPES = [
    "ALS",
    "ULS",
    "TLS",
]

subprocess.run(["cargo", "build", "--release"])

out = open("test.tsv", "w")
counter = 0

for (directory, folders, files) in os.walk(SOURCE):
    if "single_trees" not in directory:
        continue
    data_found = False
    for file in files:
        if not file.endswith(".geojson"):
            continue
        path = directory + "/" + file
        data = open(path).read()
        data = json.loads(data)
        data_found = True
        break
    if not data_found:
        continue
    data = data["properties"]["measurements"]
    data_found = False
    for entry in data:
        if "source" not in entry:
            continue
        if entry["source"] != "FI":
            continue
        target_trunk_diameter = entry["DBH_cm"]
        target_crown_height = entry["crown_base_height_m"]
        target_height = entry["height_m"]
        target_crown_diameter = entry["mean_crown_diameter_m"]
        data_found = True
        if target_height == "NA":
            target_height = ""
        if "-" in target_crown_height:
            target_crown_height = ""
        if target_crown_diameter == "NA":
            target_crown_diameter = ""
        break
    if not data_found:
        continue
    for file in files:
        if not file.endswith(".laz"):
            continue
        if all(map(lambda t: t not in file, TYPES)):
            continue
        path = directory + "/" + file
        p = subprocess.run(["./target/release/treee", "importer", path, "-o=" + OUTPUT + "/" + file[:-4], "--single-tree"], capture_output=True)
        res = p.stderr.decode()
        if res == None or res == "":
            print(p.stdout.decode())
            continue
        res = json.loads(res)
        if target_trunk_diameter != "":
            res_trunk_diameter = res["DBH_cm"]
        else:
            res_trunk_diameter = ""
        if target_crown_height != "":
            res_crown_height = res["crown_base_height_m"]
        else:
            res_crown_height = "" 
        if target_height != "":
            res_height = res["height_m"]
        else:
            res_height = "" 
        if target_crown_diameter != "":
            res_crown_diameter = res["mean_crown_diameter_m"]
        else:
            res_crown_diameter = "" 

        if "ALS" in file:
            before = ""
            after = "\t\t"
        elif "ULS" in file:
            before = "\t"
            after = "\t"
        else:
            before = "\t\t"
            after = ""

        out.write(f"{target_trunk_diameter}\t{before}{res_trunk_diameter}{after}\t")
        out.write(f"{target_crown_height}\t{before}{res_crown_height}{after}\t")
        out.write(f"{target_height}\t{before}{res_height}{after}\t")
        out.write(f"{target_crown_diameter}\t{before}{res_crown_diameter}{after}\n")
        counter += 1
        print(counter, path)