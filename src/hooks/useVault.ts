// hooks/useVault.ts
// invoke() の呼び出しはこのフックに集約する。
// Componentから直接 invoke() を呼ばない。

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type {
	CreateNoteResponse,
	NoteKind,
	VaultSummary,
	WeeklyTasks,
} from "../types";

export function useVault() {
	const [vaultRoot, setVaultRootState] = useState<string | null>(null);
	const [summary, setSummary] = useState<VaultSummary | null>(null);
	const [weeklyTasks, setWeeklyTasks] = useState<WeeklyTasks | null>(null);
	const [weekOffset, setWeekOffset] = useState(0);
	const [loading, setLoading] = useState(false);
	const [error, setError] = useState<string | null>(null);

	// 起動時にVaultパスを復元
	useEffect(() => {
		invoke<string | null>("get_vault_root").then((path) => {
			if (path) setVaultRootState(path);
		});
	}, []);

	// レースコンディション防止: 最新のリクエストのみ state を更新する
	const weeklyRequestId = useRef(0);

	const refreshWeeklyTasks = useCallback(async (offset: number) => {
		const requestId = ++weeklyRequestId.current;
		setError(null);
		try {
			// Tauri v2 は camelCase → snake_case を自動変換するため weekOffset で正しい
			const data = await invoke<WeeklyTasks>("get_weekly_tasks", {
				weekOffset: offset,
			});
			if (requestId === weeklyRequestId.current) {
				setWeeklyTasks(data);
			}
		} catch (e) {
			if (requestId === weeklyRequestId.current) {
				setError(String(e));
			}
		}
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

	// Vault設定時・weekOffset変更時に週間タスクを取得
	useEffect(() => {
		if (!vaultRoot) return;
		refreshWeeklyTasks(weekOffset);
	}, [vaultRoot, refreshWeeklyTasks, weekOffset]);

	// vault:changed リスナーから最新の weekOffset を参照するための ref
	const weekOffsetRef = useRef(weekOffset);
	useEffect(() => {
		weekOffsetRef.current = weekOffset;
	}, [weekOffset]);

	// Rustからの "vault:changed" イベントでリフレッシュ
	useEffect(() => {
		const unlisten = listen("vault:changed", () => {
			refreshSummary();
			refreshWeeklyTasks(weekOffsetRef.current);
		});
		return () => {
			unlisten.then((fn) => fn());
		};
	}, [refreshSummary, refreshWeeklyTasks]);

	const setVaultRoot = useCallback(async (path: string) => {
		await invoke("set_vault_root", { path });
		setVaultRootState(path);
	}, []);

	const getAiAvailability = useCallback(async () => {
		return invoke<boolean>("get_ai_availability");
	}, []);

	const createNote = useCallback(
		async (kind: NoteKind): Promise<CreateNoteResponse> => {
			return invoke<CreateNoteResponse>("create_note", {
				request: { kind },
			});
		},
		[],
	);

	// 週切替時にデータをリセットして表示不整合を防ぐ
	const changeWeek = useCallback((offset: number) => {
		setWeeklyTasks(null);
		setWeekOffset(offset);
	}, []);

	return {
		vaultRoot,
		summary,
		weeklyTasks,
		weekOffset,
		setWeekOffset: changeWeek,
		loading,
		error,
		setError,
		setVaultRoot,
		createNote,
		refreshSummary,
		getAiAvailability,
	};
}
