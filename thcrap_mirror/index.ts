import { assert } from 'node:console';
import fs from 'node:fs';
import path from 'node:path';
import { Readable } from 'node:stream';
import { finished } from 'node:stream/promises';
import { ReadableStream } from 'stream/web';
import crc32 from 'crc/crc32';
import retry from 'promise-fn-retry';
import { throttleAll } from 'promise-throttle-all'
import {Repo, RawRepo, OUT_DIR, Patch} from "./lib/thcrap"

function retryFetch(url: string | URL): Promise<Response>{
    const promiseFn = ()=>fetch(url);
    return retry(promiseFn, {
        times: 3,
        initialDelayTime: 100,
        onRetry: (error) => {
            console.log(error);
        },
    });
}
async function fetchRepo(root: string): Promise<Repo>{
    console.log(`Fetching ${root}`);
    const req = await retryFetch(new URL("repo.js", root));
    if(!req.ok){
        throw new Error("Error while fetching repo: "+ root);
    }
    const repo_json: RawRepo = await req.json();
    await fs.promises.mkdir(path.join(OUT_DIR, repo_json.id), {recursive: true});
    await fs.promises.writeFile(path.join(OUT_DIR, repo_json.id, "repos.js"), JSON.stringify(repo_json))
    // Prefer authoritative server.
    const servers = repo_json.servers;
    if(servers){
        servers.push(root);
        const servers_scored: [string, number][] = servers.map((x)=>{
            // Known https url.
            if(x.includes("http://mirrors.thpatch.net")){
                x=x.replace("http://mirrors.thpatch.net", "https://mirrors.thpatch.net")
            }
            // Unknown https url.
            let delta = 0;
            if(!x.includes("https://")){
                delta-=5;
            }
            if(x.includes("mirrors.thpatch.net") || x.includes("srv.thpatch.net")){
                return [x, 100+delta];
            }else if(x.includes("thpatch.rcopky.top")){
                return [x, 50+delta];
            }else{
                return [x, 0+delta];
            }
            
        });
        servers_scored.sort((x, y)=>y[1]-x[1]);
        if(root!==servers_scored[0][0]){
            console.log(`Preferring root ${servers_scored[0][0]} over ${root}`);
        }
        root = servers_scored[0][0];
    }
    return {
        url: root,
        id: repo_json.id,
        patches: Object.keys(repo_json.patches),
        raw: repo_json
    }
}
async function recursivelyFindAllRepos(root: string): Promise<Repo[]>{
    let repos = [root];
    const ret: Repo[] = [];
    const seen_repos: Record<string, string> = {};
    const seen_mirrors: Set<string> = new Set();
    while(repos.length){
        //const next = repos.pop();
        const nexts = repos.map(async next=>{
            if(!seen_mirrors.has(next)){
                seen_mirrors.add(next);
                const repo_data = await fetchRepo(next);
                if(repo_data.id in seen_repos){
                    if(repo_data.url !== seen_repos[repo_data.id]){
                        console.warn(`Patch ${repo_data.id} with different url found: ${repo_data.url} vs ${seen_repos[repo_data.id]}`)
                    }
                }else{
                    ret.push(repo_data);
                    seen_repos[repo_data.id] = repo_data.url;
                    for(const neighbour of repo_data.raw.neighbors ?? []){
                        repos.push(neighbour);
                    }
                }

            }
        });
        repos = [];
        await Promise.all(nexts);
    }
    return ret;
}
async function fetchFileFromPatch(root: Patch, file: string, crc: number|null, server: URL = root.base){
    if(crc===null){
        //console.log(`Skipping file ${root.repo.id}/${root.id}/${file}`);
        return;
    }
    const out_path = path.join(OUT_DIR, root.repo.id, root.id, file);
    if(await fs.existsSync(out_path)){
        const buffer = await fs.promises.readFile(out_path);
        const actual = crc32(buffer);
        if(actual===crc){
            //console.warn(`CRC32 matched for ${root.repo.id}/${root.id}/${file}. Skipping`);
        }
        return {out: out_path, crc: actual};
    }
    console.log(`Fetching file ${root.repo.id}/${root.id}/${file}`);
    const file_url = new URL(file, root.base);
    return await retry(async ()=>{
        const req = await fetch(file_url);
        if(!req.ok){
            console.error(`Error while fetching file: ${file_url}`);
            return;
        }
        
        const body = req.body as ReadableStream<any>
        if(!body){
            throw new Error(`Error while fetching file body: ${file_url}`);
        }
        await fs.promises.mkdir(path.dirname(out_path), {recursive: true});
        const w = fs.createWriteStream(out_path);
        await finished(Readable.fromWeb(body).pipe(w));
        // crc32
        const buffer = await fs.promises.readFile(out_path);
        const actual = crc32(buffer);
        if(actual!==crc){
            console.warn(`CRC32 mismatch for ${file_url}\nexpected: ${crc}\n  actual: ${actual}`)
            
        }
        // confidently using our crc32 instead of theirs.
        return {out: out_path, crc: actual};
    }, {
        times: 10,
        initialDelayTime: 100,
        onRetry: (error) => {
            console.log(error);
        },
    })
    
}
async function fetchPatch(root: Repo, name: string): Promise<Patch>{
    //console.log(`Fetching patch ${root.id}/${name}`);
    const patch_base= new URL(name+"/", root.url);
    const files_js_url = new URL("files.js", patch_base);
    
    const req = await retryFetch(files_js_url);
    if(!req.ok){
        throw new Error(`Error while fetching patch ${root.raw.id}/${name} (url=${files_js_url})`);
    }
    const files_js: Record<string, number> = await req.json();
    const patch_path = path.join(OUT_DIR, root.id, name);
    await fs.promises.mkdir(patch_path, {recursive: true});
    
    const semi_patch: Patch = {
        repo: root,
        id: name,
        base: patch_base,
        files: files_js,
        patchFileMirrors: []
    };
    if(!("patch.js" in files_js)){
        throw new Error("patch.js not found in: "+ root.raw.id+"/"+name);
    }
    const {out: patch_metadata_path, crc} = (await fetchFileFromPatch(semi_patch, "patch.js", files_js["patch.js"]))!;
    files_js["patch.js"] = crc;
    const patch_metadata = JSON.parse(await fs.promises.readFile(patch_metadata_path, {encoding: "utf-8"}));
    semi_patch.patchFileMirrors = patch_metadata.servers || [];
    await(fs.promises.writeFile(path.join(patch_path, "files.js"), JSON.stringify(files_js)));
    //console.log(`Fetched patch ${root.id}/${name}`);
    return semi_patch;
    
}


(async ()=>{
    
    await fs.promises.mkdir(OUT_DIR, {recursive: true});
    const repos = await recursivelyFindAllRepos("https://srv.thpatch.net/");
    console.log(repos.length);
    const patch_fetch : Promise<Patch>[] = [];
    for(const repo of repos){
        for(const patch of repo.patches){
            patch_fetch.push(fetchPatch(repo, patch));
        }
    }
    const patches = await Promise.all(patch_fetch);
    for(const patch of patches){
        const current_patch = []
        for(const [f, crc] of Object.entries(patch.files)){
            current_patch.push(()=>fetchFileFromPatch(patch, f, crc));
            //current_patch.push();
        }
        await throttleAll(4, current_patch);
        //await Promise.all(current_patch)
    }
})();
