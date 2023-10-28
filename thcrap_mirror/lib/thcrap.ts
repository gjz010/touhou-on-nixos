import path from 'node:path'
export const OUT_DIR = (()=>{
    let outdir = process.env["THCRAP_MIRROR_OUT"];
    if(!outdir){
        outdir = path.join(process.cwd(), "repos");
    }
    return outdir;
})();
export interface RawRepo{
    id: string,
    title?: string,
    contact?: string,
    patches: Record<string, string>,
    patchdata?: any,
    flags?: any,
    games?: any,
    dependencies?: any,
    servers?: string[],
    neighbors?: string[]
}


export interface Repo{
    url: string,
    id: string,
    patches: string[],
    raw: RawRepo
}

export interface Patch{
    repo: Repo,
    id: string,
    base: URL,
    files: Record<string, number>,
    patchFileMirrors: string[]
}
