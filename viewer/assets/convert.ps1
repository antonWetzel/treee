cd $PSScriptRoot
$files = Get-ChildItem -Path "." -Filter *.svg


foreach ($file in $files) {
	echo $file
	$name = $file.Name.Replace("svg", "png")
	# inkscape -b white -h 64 $file -o $name
	inkscape -w 256 $file -o $name
}

inkscape -h 256 line.svg -o line.png
inkscape -h 256 tree-fill-bg.svg -o tree-fill-big.png
inkscape -h 16 tree-fill-bg.svg -o tree-fill-small.png
