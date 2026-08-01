import { useCallback, useMemo, useState } from "preact/hooks";
import type { WorkspaceFile } from "../types.ts";
import type { Translations } from "../i18n/translations.ts";

type ExplorerNode = ExplorerFolder | ExplorerFile;

interface ExplorerFolder {
  kind: "folder";
  name: string;
  path: string;
  children: ExplorerNode[];
}

interface ExplorerFile {
  kind: "file";
  file: WorkspaceFile;
  path: string;
}

interface ExplorerProps {
  files: WorkspaceFile[];
  activeFileId: string;
  renamingId: string | null;
  renameValue: string;
  renameRef: { current: HTMLInputElement | null };
  t: Translations;
  onCreateFile: () => void;
  onOpenFile: (id: string) => void;
  onBeginRename: (file: WorkspaceFile, event: Event) => void;
  onDeleteFile: (file: WorkspaceFile, event: Event) => void;
  onResetFile: (file: WorkspaceFile, event: Event) => void;
  onCommitRename: () => void;
  onRenameValueChange: (value: string) => void;
  onCancelRename: () => void;
  onResetWorkspace: () => void;
}

function buildTree(files: WorkspaceFile[]): ExplorerFolder {
  const root: ExplorerFolder = { kind: "folder", name: "", path: "", children: [] };

  for (const file of files) {
    const parts = file.name.split("/").filter(Boolean);
    if (parts.length === 0) continue;
    let folder = root;
    for (const part of parts.slice(0, -1)) {
      const path = folder.path ? `${folder.path}/${part}` : part;
      let child = folder.children.find((node): node is ExplorerFolder => node.kind === "folder" && node.name === part);
      if (!child) {
        child = { kind: "folder", name: part, path, children: [] };
        folder.children.push(child);
      }
      folder = child;
    }
    folder.children.push({ kind: "file", file, path: file.name });
  }

  const sortNodes = (nodes: ExplorerNode[]) => {
    nodes.sort((a, b) => {
      if (a.kind !== b.kind) return a.kind === "folder" ? -1 : 1;
      return (a.kind === "folder" ? a.name : a.file.name).localeCompare(b.kind === "folder" ? b.name : b.file.name);
    });
    for (const node of nodes) if (node.kind === "folder") sortNodes(node.children);
  };
  sortNodes(root.children);
  return root;
}

function initialExpandedFolders(files: WorkspaceFile[]): Set<string> {
  const expanded = new Set<string>();
  for (const parts of files.map((file) => file.name.split("/").slice(0, -1))) {
    for (let index = 1; index <= parts.length; index++) expanded.add(parts.slice(0, index).join("/"));
  }
  return expanded;
}

export function Explorer({
  files,
  activeFileId,
  renamingId,
  renameValue,
  renameRef,
  t,
  onCreateFile,
  onOpenFile,
  onBeginRename,
  onDeleteFile,
  onResetFile,
  onCommitRename,
  onRenameValueChange,
  onCancelRename,
  onResetWorkspace,
}: ExplorerProps) {
  const tree = useMemo(() => buildTree(files), [files]);
  const [expanded, setExpanded] = useState<Set<string>>(() => initialExpandedFolders(files));

  const toggleFolder = useCallback((path: string) => {
    setExpanded((current) => {
      const next = new Set(current);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }, []);

  const renderFile = (node: ExplorerFile, depth: number) => {
    const file = node.file;
    return (
      <div key={file.id} class={"file-row tree-file" + (activeFileId === file.id ? " active" : "")}>
        {renamingId === file.id ? (
          <input
            ref={renameRef}
            class="file-rename-input"
            style={{ marginLeft: `${8 + depth * 16}px` }}
            value={renameValue}
            aria-label={t.renameFile}
            onInput={(event) => onRenameValueChange((event.target as HTMLInputElement).value)}
            onKeyDown={(event) => {
              if (event.key === "Enter") onCommitRename();
              if (event.key === "Escape") onCancelRename();
            }}
            onBlur={onCommitRename}
          />
        ) : (
          <button class="file-open" style={{ paddingLeft: `${8 + depth * 16}px` }} onClick={() => onOpenFile(file.id)}>
            <span class="js-icon">JS</span>
            <span class="file-name">{file.name.split("/").at(-1)}</span>
            {file.readonly && <span class="file-readonly" title={t.readOnlyFile}>•</span>}
            {file.dirty && <span class="file-dirty">●</span>}
          </button>
        )}
        {renamingId !== file.id && !file.readonly && (
          <span class="file-actions">
            {file.dirty && <button onClick={(event) => onResetFile(file, event)} title={t.restoreFile} aria-label={t.restoreFile}>↶</button>}
            <button onClick={(event) => onBeginRename(file, event)} title={t.renameFile} aria-label={t.renameFile}>✎</button>
            <button onClick={(event) => onDeleteFile(file, event)} title={t.deleteFile} aria-label={t.deleteFile}>×</button>
          </span>
        )}
      </div>
    );
  };

  const renderNodes = (nodes: ExplorerNode[], depth = 0): preact.JSX.Element[] => nodes.map((node) => {
    if (node.kind === "file") return renderFile(node, depth);
    const isExpanded = expanded.has(node.path);
    return (
      <div key={node.path} class="tree-folder">
        <button class="tree-folder-row" style={{ paddingLeft: `${8 + depth * 16}px` }} onClick={() => toggleFolder(node.path)} aria-expanded={isExpanded}>
          <span class="tree-chevron">{isExpanded ? "⌄" : "›"}</span>
          <span class="folder-icon">▱</span>
          <span class="tree-folder-name">{node.name}</span>
        </button>
        {isExpanded && <div class="tree-folder-children">{renderNodes(node.children, depth + 1)}</div>}
      </div>
    );
  });

  return (
    <div class="sidebar-section files-section">
      <div class="sidebar-heading">
        <span>{t.explorer}</span>
        <span class="sidebar-actions">
          <button class="sidebar-action" onClick={onCreateFile} title={t.newFile} aria-label={t.newFile}>＋</button>
          <button class="sidebar-action" onClick={onResetWorkspace} title={t.resetWorkspace} aria-label={t.resetWorkspace}>↺</button>
        </span>
      </div>
      <div class="workspace-root"><span class="chevron">⌄</span><span class="folder-icon">▱</span><span>{t.workspace}</span></div>
      <div class="sidebar-heading files-heading"><span>{t.files}</span><span class="count-badge">{files.length}</span></div>
      <div class="file-tree">{renderNodes(tree.children)}</div>
    </div>
  );
}
