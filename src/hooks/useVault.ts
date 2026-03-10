// hooks/useVault.ts
// invoke() の呼び出しはこのフックに集約する。
// Componentから直接 invoke() を呼ばない。

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useState } from "react";
import type { CreateNoteResponse, NoteKind, VaultSummary } from "../types";

export function useVault() {
	const [vaultRoot, setVaultRootState] = useState<string | null>(null);
	const [summary, setSummary] = useState<VaultSummary | null>(null);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	// 起動時にVaultパスを復元
	useEffect(() => {
		invoke<string | null>("get_vault_root").then((path) => {
			if (path) setVaultRootState(path);
		});
	}, []);

	const refreshSummary = useCallback(async () => {
		setLoading(true);
		setError(null);
		try {
			const data = await invoke<VaultSummary>("get_vault_summary");
			setSummary(data);
		} catch (e) {
			setError(String(e));
		} finally {
			setLoading(false);
		}
	}, []);

	// Vaultが設定されたらサマリーを取得
	useEffect(() => {
		if (!vaultRoot) return;
		refreshSummary();
	}, [vaultRoot, refreshSummary]);

	// Rustからの "vault:changed" イベントでリフレッシュ
	useEffect(() => {
		const unlisten = listen("vault:changed", () => {
			refreshSummary();
		});
		return () => {
			unlisten.then((fn) => fn());
		};
	}, [refreshSummary]);

	const setVaultRoot = useCallback(async (path: string) => {
		await invoke("set_vault_root", { path });
		setVaultRootState(path);
	}, []);

	const createNote = useCallback(
		async (kind: NoteKind): Promise<CreateNoteResponse> => {
			return invoke<CreateNoteResponse>("create_note", {
				request: { kind },
			});
		},
		[],
	);

	return {
		vaultRoot,
		summary,
		loading,
		error,
		setError,
		setVaultRoot,
		createNote,
		refreshSummary,
	};
}
