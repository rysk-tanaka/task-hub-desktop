import type { ProjectProgress, Task, VaultSummary } from "../types";

function StatCard({
	label,
	value,
	accent = false,
}: {
	label: string;
	value: number;
	accent?: boolean;
}) {
	return (
		<div
			style={{
				background: accent ? "var(--accent-bg)" : "var(--card-bg)",
				border: `1px solid ${accent ? "var(--accent)" : "var(--border)"}`,
				borderRadius: 8,
				padding: "16px 20px",
				minWidth: 120,
			}}
		>
			<div
				style={{
					fontSize: "var(--font-xl)",
					fontWeight: 700,
					color: accent ? "var(--accent)" : "var(--text)",
				}}
			>
				{value}
			</div>
			<div
				style={{
					fontSize: "var(--font-sm)",
					color: "var(--text-muted)",
					marginTop: 2,
				}}
			>
				{label}
			</div>
		</div>
	);
}

function TaskRow({ task }: { task: Task }) {
	const todayStr = new Date().toLocaleDateString("sv-SE");
	const isOverdue = task.due != null && task.due < todayStr;
	return (
		<div
			style={{
				display: "flex",
				alignItems: "baseline",
				gap: 10,
				padding: "6px 0",
				borderBottom: "1px solid var(--border)",
				fontSize: "var(--font-base)",
			}}
		>
			<span
				style={{
					color: isOverdue ? "var(--danger)" : "var(--text-muted)",
					minWidth: 86,
				}}
			>
				{task.due ?? "\u2014"}
			</span>
			<span style={{ color: "var(--text)", flex: 1 }}>{task.text}</span>
			<span
				style={{
					fontSize: "var(--font-base)",
					color: "var(--text-muted)",
					maxWidth: 160,
					overflow: "hidden",
					textOverflow: "ellipsis",
					whiteSpace: "nowrap",
				}}
			>
				{task.source_file}
			</span>
		</div>
	);
}

function ProgressBar({ percent }: { percent: number }) {
	return (
		<div
			style={{
				height: 4,
				background: "var(--border)",
				borderRadius: 2,
				overflow: "hidden",
				flex: 1,
			}}
		>
			<div
				style={{
					height: "100%",
					width: `${percent}%`,
					background: percent === 100 ? "var(--success)" : "var(--accent)",
					borderRadius: 2,
					transition: "width 0.3s ease",
				}}
			/>
		</div>
	);
}

function ProjectRow({ project }: { project: ProjectProgress }) {
	return (
		<div
			style={{
				display: "flex",
				alignItems: "center",
				gap: 12,
				padding: "7px 0",
				borderBottom: "1px solid var(--border)",
				fontSize: "var(--font-base)",
			}}
		>
			<span style={{ flex: 1, color: "var(--text)" }}>{project.name}</span>
			<ProgressBar percent={project.percent} />
			<span
				style={{ color: "var(--text-muted)", minWidth: 60, textAlign: "right" }}
			>
				{project.completed}/{project.total}
			</span>
		</div>
	);
}

export function SummaryView({ summary }: { summary: VaultSummary }) {
	return (
		<>
			{/* サマリーカード */}
			<section style={styles.section}>
				<div style={{ display: "flex", gap: 12, flexWrap: "wrap" }}>
					<StatCard
						label="Inbox"
						value={summary.inbox_count}
						accent={summary.inbox_count > 0}
					/>
					<StatCard
						label="今日の期限"
						value={summary.due_today.length}
						accent={summary.due_today.length > 0}
					/>
					<StatCard
						label="期限超過"
						value={summary.overdue.length}
						accent={summary.overdue.length > 0}
					/>
					<StatCard label="プロジェクト" value={summary.projects.length} />
				</div>
			</section>

			<div style={styles.columns}>
				{/* 左カラム: タスク */}
				<div style={{ flex: 1, minWidth: 0 }}>
					{summary.overdue.length > 0 && (
						<section style={styles.section}>
							<h3 style={styles.sectionTitle}>🔴 期限超過</h3>
							{summary.overdue.map((t) => (
								<TaskRow key={`${t.source_file}:${t.line}`} task={t} />
							))}
						</section>
					)}

					{summary.due_today.length > 0 && (
						<section style={styles.section}>
							<h3 style={styles.sectionTitle}>📅 今日の期限</h3>
							{summary.due_today.map((t) => (
								<TaskRow key={`${t.source_file}:${t.line}`} task={t} />
							))}
						</section>
					)}

					{summary.due_today.length === 0 && summary.overdue.length === 0 && (
						<section style={styles.section}>
							<div
								style={{
									color: "var(--text-muted)",
									fontSize: "var(--font-base)",
									padding: "12px 0",
								}}
							>
								期限タスクはありません ✅
							</div>
						</section>
					)}
				</div>

				{/* 右カラム: プロジェクト進捗 */}
				<div style={{ width: 320, flexShrink: 0 }}>
					<section style={styles.section}>
						<h3 style={styles.sectionTitle}>📁 プロジェクト</h3>
						{summary.projects.length === 0 ? (
							<div
								style={{
									color: "var(--text-muted)",
									fontSize: "var(--font-base)",
									padding: "12px 0",
								}}
							>
								進行中のプロジェクトはありません
							</div>
						) : (
							summary.projects.map((p) => (
								<ProjectRow key={p.file} project={p} />
							))
						)}
					</section>
				</div>
			</div>
		</>
	);
}

const styles: Record<string, React.CSSProperties> = {
	section: {
		marginBottom: 20,
	},
	sectionTitle: {
		fontSize: "var(--font-sm)",
		fontWeight: 600,
		color: "var(--text-muted)",
		textTransform: "uppercase",
		letterSpacing: "0.06em",
		marginBottom: 8,
	},
	columns: {
		display: "flex",
		gap: 20,
		marginTop: 16,
		alignItems: "flex-start",
	},
};
