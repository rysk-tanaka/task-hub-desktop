import type { TaskStatus, WeeklyTasks } from "../types";

const STATUS_MARKER: Record<TaskStatus, string> = {
	todo: "\u2b1c",
	done: "\u2705",
	in_progress: "\ud83d\udd04",
	waiting: "\u23f8\ufe0f",
	cancelled: "\u274c",
};

function formatRange(start: string, end: string): string {
	const s = new Date(`${start}T00:00:00`);
	const e = new Date(`${end}T00:00:00`);
	return `${s.getMonth() + 1}/${s.getDate()} ~ ${e.getMonth() + 1}/${e.getDate()}`;
}

export function WeeklySection({
	weeklyTasks,
	weekOffset,
	onChangeWeek,
}: {
	weeklyTasks: WeeklyTasks;
	weekOffset: number;
	onChangeWeek: (offset: number) => void;
}) {
	const weekLabel =
		weekOffset === 0
			? "今週"
			: weekOffset === -1
				? "先週"
				: weekOffset === 1
					? "来週"
					: `${weekOffset > 0 ? "+" : ""}${weekOffset}w`;

	return (
		<section style={{ marginBottom: 20 }}>
			<div
				style={{
					display: "flex",
					alignItems: "center",
					gap: 12,
					marginBottom: 12,
				}}
			>
				<h3 style={styles.sectionTitle}>今週の予定</h3>
				<div style={{ display: "flex", alignItems: "center", gap: 4 }}>
					<button
						type="button"
						style={styles.navButton}
						onClick={() => onChangeWeek(weekOffset - 1)}
					>
						&#9664;
					</button>
					<span
						style={{
							fontSize: 12,
							color: "var(--text-muted)",
							minWidth: 40,
							textAlign: "center",
						}}
					>
						{weekLabel}
					</span>
					<button
						type="button"
						style={styles.navButton}
						onClick={() => onChangeWeek(weekOffset + 1)}
					>
						&#9654;
					</button>
				</div>
				<span style={{ fontSize: 12, color: "var(--text-muted)" }}>
					{formatRange(weeklyTasks.week_start, weeklyTasks.week_end)}
				</span>
			</div>

			{weeklyTasks.projects.length === 0 ? (
				<div
					style={{
						color: "var(--text-muted)",
						fontSize: 13,
						padding: "12px 0",
					}}
				>
					タスクはありません
				</div>
			) : (
				weeklyTasks.projects.map((project) => (
					<div key={project.file} style={{ marginBottom: 16 }}>
						<h4 style={styles.projectName}>{project.name}</h4>
						{project.tasks.map((task) => (
							<div
								key={`${task.source_file}:${task.line}`}
								style={styles.taskRow}
							>
								<span style={{ width: 20, textAlign: "center", flexShrink: 0 }}>
									{STATUS_MARKER[task.status]}
								</span>
								<span
									style={{
										flex: 1,
										color:
											task.status === "done" || task.status === "cancelled"
												? "var(--text-muted)"
												: "var(--text)",
										textDecoration:
											task.status === "done" ? "line-through" : "none",
									}}
								>
									{task.text}
								</span>
								{task.start && (
									<span
										style={{
											fontSize: 11,
											color: "var(--text-muted)",
											flexShrink: 0,
										}}
									>
										{task.start}
									</span>
								)}
							</div>
						))}
					</div>
				))
			)}
		</section>
	);
}

const styles: Record<string, React.CSSProperties> = {
	sectionTitle: {
		fontSize: 13,
		fontWeight: 600,
		color: "var(--text-muted)",
		textTransform: "uppercase",
		letterSpacing: "0.06em",
		margin: 0,
	},
	navButton: {
		background: "transparent",
		border: "1px solid var(--border)",
		borderRadius: 4,
		color: "var(--text-muted)",
		cursor: "pointer",
		fontSize: 10,
		padding: "2px 6px",
		lineHeight: 1,
	},
	projectName: {
		fontSize: 13,
		fontWeight: 600,
		color: "var(--accent)",
		margin: "0 0 6px 0",
	},
	taskRow: {
		display: "flex",
		alignItems: "baseline",
		gap: 8,
		padding: "4px 0 4px 8px",
		borderBottom: "1px solid var(--border)",
		fontSize: 13,
	},
};
