// Thin wrapper over the Tauri commands in src-tauri/src/lib.rs.
const invoke = (cmd, args) => {
  const tauri = window.__TAURI__;
  if (!tauri) return Promise.reject(new Error('Not running inside the Buckets app'));
  return tauri.core.invoke(cmd, args);
};

export const api = {
  status: () => invoke('app_status'),
  board: () => invoke('board'),
  files: (bucketId) => invoke('bucket_files', { bucketId }),
  history: (bucketId) => invoke('bucket_history', { bucketId }),
  commitChanges: (bucketId, commitId) => invoke('commit_changes', { bucketId, commitId }),
  activity: (limit = 150) => invoke('activity', { limit }),
  preview: (bucketId, path, source) => invoke('file_preview', { bucketId, path, source }),
  diff: (bucketId, path, left, right) => invoke('file_diff', { bucketId, path, left, right }),
  action: (action) => invoke('run_action', { action }),
  open: (bucketId, path = null, reveal = false) => invoke('open_file', { bucketId, path, reveal }),
  chooseRepo: () => invoke('choose_repository'),
  openRepo: (path) => invoke('open_repository', { path }),
  settings: (patch) => invoke('update_settings', { patch }),
};

export const working = { kind: 'working' };
export const object = (hash) => ({ kind: 'object', hash });
