// Custom Effects manager: list/upload/delete .js effect scripts, plus the
// authoring resources (AI prompt download, docs pointers).

import aiPrompt from "../../../docs/ai-effect-prompt.txt?raw";
import { core } from "../ipc";
import { refreshStatus, store } from "../state";
import { sfx } from "../sound";

async function refreshEffectsList() {
  const fx = await core.req("effects");
  if (fx.ok) {
    store.effects = fx.effects;
    store.emit();
  }
}

function toast(msg: string) {
  const root = document.getElementById("toast-root")!;
  const t = document.createElement("div");
  t.className = "toast";
  t.textContent = msg;
  root.appendChild(t);
  setTimeout(() => t.remove(), 4200);
}

export function renderCustom(root: HTMLElement): (() => void) | void {
  root.innerHTML = "";
  const view = document.createElement("div");
  view.className = "view";

  // ---------- header / actions
  const head = document.createElement("div");
  head.className = "panel";
  head.innerHTML = `<h3>Custom effects</h3>
    <div class="sub">Your own effects, written in JavaScript and run on Keyscape's embedded
    engine — nothing to install. Add a <code>.js</code> file here and it appears in the Effects
    gallery instantly (no restart needed). New to this? Open the <b>Guide</b> tab — it has a full
    tutorial, or let an AI write the effect for you with the prompt file below.</div>`;

  const actions = document.createElement("div");
  actions.style.cssText = "display:flex;gap:8px;flex-wrap:wrap;margin-top:4px";

  const fileInput = document.createElement("input");
  fileInput.type = "file";
  fileInput.accept = ".js";
  fileInput.style.display = "none";

  const addBtn = document.createElement("button");
  addBtn.className = "btn primary";
  addBtn.textContent = "＋ Add effect file";
  addBtn.addEventListener("click", () => {
    sfx.click();
    fileInput.click();
  });
  fileInput.addEventListener("change", async () => {
    const f = fileInput.files?.[0];
    if (!f) return;
    const content = await f.text();
    const r = await core.req("save_script", { name: f.name, content });
    if (r.ok) {
      sfx.select();
      toast(`Added ${r.file} — it's live in the gallery`);
      await refreshEffectsList();
      renderCustom(root);
    } else {
      toast(`Rejected: ${r.error}`);
    }
    fileInput.value = "";
  });

  const reloadBtn = document.createElement("button");
  reloadBtn.className = "btn";
  reloadBtn.textContent = "↻ Reload scripts";
  reloadBtn.addEventListener("click", async () => {
    sfx.click();
    await core.req("rescan_scripts");
    await refreshEffectsList();
    renderCustom(root);
    toast("Scripts rescanned");
  });

  const folderBtn = document.createElement("button");
  folderBtn.className = "btn";
  folderBtn.textContent = "📂 Open effects folder";
  folderBtn.addEventListener("click", () => {
    sfx.click();
    void core.req("open_effects_dir");
  });

  const promptBtn = document.createElement("button");
  promptBtn.className = "btn";
  promptBtn.textContent = "🤖 Download AI prompt (.txt)";
  promptBtn.title = "Give this file to ChatGPT/Claude/any AI plus your effect idea";
  promptBtn.addEventListener("click", () => {
    sfx.click();
    const blob = new Blob([aiPrompt], { type: "text/plain" });
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = "keyscape-ai-effect-prompt.txt";
    a.click();
    URL.revokeObjectURL(a.href);
    toast("Paste the file into any AI chat + describe your effect");
  });

  const guideBtn = document.createElement("button");
  guideBtn.className = "btn";
  guideBtn.textContent = "✦ Authoring guide";
  guideBtn.addEventListener("click", () => {
    document.querySelector<HTMLElement>('.nav-btn[data-view="guide"]')?.click();
  });

  actions.append(addBtn, reloadBtn, folderBtn, promptBtn, guideBtn);
  head.appendChild(actions);
  head.appendChild(fileInput);
  view.appendChild(head);

  // ---------- installed scripts
  const listPanel = document.createElement("div");
  listPanel.className = "panel";
  listPanel.style.marginTop = "14px";
  listPanel.innerHTML = `<h3>Installed scripts</h3><div class="sub" id="scripts-sub">Loading…</div>`;
  const listBody = document.createElement("div");
  listPanel.appendChild(listBody);
  view.appendChild(listPanel);
  root.appendChild(view);

  void (async () => {
    const r = await core.req("scripts");
    const sub = listPanel.querySelector("#scripts-sub")!;
    if (!r.ok) {
      sub.textContent = "Core offline.";
      return;
    }
    const scripts: { file: string; id?: string; name?: string; error?: string }[] = r.scripts;
    sub.innerHTML = `${scripts.length || "No"} file(s) in <code>${r.dir}</code>`;
    for (const s of scripts) {
      const row = document.createElement("div");
      row.className = "row";
      const ok = !s.error;
      row.innerHTML = `<div><div class="lbl">${s.file} ${
        ok
          ? `<span class="badge-input" style="background:rgba(34,211,165,.2);border-color:rgba(34,211,165,.4);color:#7dedc9">loaded</span>`
          : `<span class="badge-input" style="background:rgba(255,84,112,.2);border-color:rgba(255,84,112,.4);color:#ff9db0">error</span>`
      }</div>
      <div class="hint">${ok ? `Appears in the gallery as “${s.name}” (id <code>${s.id}</code>)` : s.error}</div></div>`;
      const btns = document.createElement("div");
      btns.style.cssText = "display:flex;gap:8px;flex:none";
      if (ok) {
        const tryBtn = document.createElement("button");
        tryBtn.className = "btn";
        tryBtn.textContent = "Try";
        tryBtn.addEventListener("click", async () => {
          sfx.select();
          await core.req("set_effect", { id: s.id });
          setTimeout(() => void refreshStatus(), 250);
        });
        btns.appendChild(tryBtn);
      }
      const delBtn = document.createElement("button");
      delBtn.className = "btn danger";
      delBtn.textContent = "Delete";
      delBtn.addEventListener("click", async () => {
        sfx.click();
        const rr = await core.req("delete_script", { file: s.file });
        if (rr.ok) {
          toast(`Deleted ${s.file}`);
          await refreshEffectsList();
          renderCustom(root);
        } else {
          toast(`Delete failed: ${rr.error}`);
        }
      });
      btns.appendChild(delBtn);
      row.appendChild(btns);
      listBody.appendChild(row);
    }
  })();
}
