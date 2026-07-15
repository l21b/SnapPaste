import { readFile } from "node:fs/promises";

const packageJson = JSON.parse(await readFile("package.json", "utf8"));
const tauriConfig = JSON.parse(
    await readFile("src-tauri/tauri.conf.json", "utf8"),
);
const cargoToml = await readFile("src-tauri/Cargo.toml", "utf8");
const cargoVersion = cargoToml.match(
    /^version\s*=\s*"([^"]+)"/m,
)?.[1];

const versions = {
    "package.json": packageJson.version,
    "src-tauri/tauri.conf.json": tauriConfig.version,
    "src-tauri/Cargo.toml": cargoVersion,
};
const uniqueVersions = new Set(Object.values(versions));

if (uniqueVersions.size !== 1 || uniqueVersions.has(undefined)) {
    console.error("Version mismatch:", versions);
    process.exit(1);
}

const tagIndex = process.argv.indexOf("--tag");
if (tagIndex >= 0) {
    const tag = process.argv[tagIndex + 1]?.replace(/^v/, "");
    if (!tag || tag !== packageJson.version) {
        console.error(
            `Release tag ${process.argv[tagIndex + 1] ?? "<missing>"} does not match version ${packageJson.version}.`,
        );
        process.exit(1);
    }
}

console.log(`Version ${packageJson.version} is consistent across project files.`);
