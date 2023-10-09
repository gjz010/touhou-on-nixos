#!/bin/sh

declare -A dlls=(
    [d3d9]="dxvk/d3d9.dll"
    [d3d10]="dxvk/d3d10.dll dxvk/d3d10_1.dll dxvk/d3d10core.dll"
    [d3d11]="dxvk/d3d11.dll"
    [dxgi]="dxvk/dxgi.dll"
    [mcfgthreads]="mcfgthreads/mcfgthread-12.dll"
)
declare -A targets=([d3d9]=1 [d3d11]=1 [dxgi]=1 [mcfgthreads]=1)

install_file() {
    $do_symlink && file_cmd="ln -sv" || file_cmd="install -m 755 -v"

    srcfile=$1
    dstfile=$2

    if [ -f "${srcfile}.so" ]; then
        srcfile="${srcfile}.so"
    fi

    if ! [ -f "${srcfile}" ]; then
        echo "${srcfile}: File not found. Skipping." >&2
        return 1
    fi

    if [ -n "$1" ]; then
        if [ -f "${dstfile}" ] || [ -h "${dstfile}" ]; then
            if ! [ -f "${dstfile}.old" ]; then
                mv -v "${dstfile}" "${dstfile}.old"
            else
                rm -v "${dstfile}"
            fi
        fi
        $file_cmd "${srcfile}" "${dstfile}"
    else
        echo "${dstfile}: File not found in wine prefix" >&2
        return 1
    fi
}


install_override() {
    dll=$(basename "$1")
    if ! wine reg add 'HKEY_CURRENT_USER\Software\Wine\DllOverrides' /v "$dll" /d native /f >/dev/null 2>&1; then
        echo -e "Failed to add override for $dll"
        exit 1
    fi
}
declare -A paths
for target in "${!targets[@]}"; do
    [ "${targets[$target]}" -eq 1 ] || continue
    for dll in ${dlls[$target]}; do
        dllname=$(basename "$dll")
        basedir=$(dirname "$dll")
        basedir32=${basedir}32_dir
        paths["${!basedir32}/$dllname"]="$WINEPREFIX/drive_c/windows/system32/$dllname"
    done
done
for srcpath in "${!paths[@]}"; do
    install_file "$srcpath" "${paths["$srcpath"]}"
    install_override "$(basename "$srcpath" .dll)"
done

