$path =  $PSScriptRoot + "/svg/"
$out_path =$PSScriptRoot + "/png/"

$files = Get-ChildItem -Path $path -Filter *.svg

foreach ($file in $files) {
	$in = $path + $file.Name
	$out = $out_path + $file.Name.Replace("svg", "png")
	inkscape -w 256 $in -o $out
}


inkscape -h 256 "$($path + "line.svg")" -o "$($out_path + "line.png")"
inkscape -h 256 "$($path + "tree-fill-bg.svg")" -o "$($out_path + "tree-fill-big.png")"
inkscape -h 16 "$($path + "tree-fill-bg.svg")" -o "$($out_path + "tree-fill-small.png")"
