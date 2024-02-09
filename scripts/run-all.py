import os
import subprocess

SOURCE = "D:/data/"
OUTPUT = "D:/projects/treee/cwd/auto/"

TYPES = [
    "ALS",
    "ULS",
    # "TLS",
]


for (directory, folders, files) in os.walk(SOURCE):
    if "single_trees" in directory:
        continue
    for file in files:
        if not file.endswith(".laz"):
            continue
        if all(map(lambda t: t not in file, TYPES)):
            continue
        path = directory + "/" + file
        print(path)
        subprocess.run(["treee", "importer", path, "-o=" + OUTPUT + file[:-4]])
