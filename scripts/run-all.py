import os
import subprocess

SOURCE = "D:/data"
OUTPUT = "D:/projects/treee/cwd/auto/"

for (directory, folders, files) in os.walk(SOURCE):
    if "single_trees" in directory:
        continue
    for file in files:
        if not file.endswith(".laz"):
            continue
        # if not "ALS" in file:
        #     continue
        if not "ULS" in file:
            continue
        path = directory + "/" + file
        subprocess.run(["treee", "importer", path, "-o=" + OUTPUT + file[:-4]])