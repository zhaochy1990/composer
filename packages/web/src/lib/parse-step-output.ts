export interface PrMeta {
    merge_conflicts: boolean;
    conflict_files: string[];
    changed_files: { status: string; path: string }[];
}

export interface TestResults {
    all_passed: boolean;
    summary: string;
}

const PR_META_RE = /<!--COMPOSER:PR_META\s*(\{.*?\})\s*-->/;
const TEST_RESULTS_RE = /<!--COMPOSER:TEST_RESULTS\s*(\{.*?\})\s*-->/;
const ALL_COMPOSER_MARKERS_RE = /<!--COMPOSER:\w+\s*\{.*?\}\s*-->\n?/g;

export function parsePrMeta(output: string): PrMeta | null {
    const match = output.match(PR_META_RE);
    if (!match) return null;
    try {
        return JSON.parse(match[1]) as PrMeta;
    } catch {
        return null;
    }
}

export function parseTestResults(output: string): TestResults | null {
    const match = output.match(TEST_RESULTS_RE);
    if (!match) return null;
    try {
        return JSON.parse(match[1]) as TestResults;
    } catch {
        return null;
    }
}

export function stripComposerMarkers(output: string): string {
    return output.replace(ALL_COMPOSER_MARKERS_RE, '').trim();
}

const STATUS_LABELS: Record<string, string> = {
    M: 'Modified',
    A: 'Added',
    D: 'Deleted',
    R: 'Renamed',
};

function statusLabel(code: string): string {
    return STATUS_LABELS[code] ?? code;
}

/**
 * Build a single markdown string for the review panel by assembling
 * structured sections from parsed step outputs.
 */
export function buildReviewMarkdown(
    prMeta: PrMeta | null,
    testResults: TestResults | null,
    reviewContent: string,
): string {
    const sections: string[] = [];

    // 1. Merge Conflict Status
    if (prMeta) {
        if (prMeta.merge_conflicts) {
            const files = prMeta.conflict_files.length > 0
                ? prMeta.conflict_files.map(f => `- \`${f}\``).join('\n')
                : '';
            sections.push(`## Merge Conflict Status\n\u274c **Merge conflicts detected**\n${files}`);
        } else {
            sections.push('## Merge Conflict Status\n\u2705 No merge conflicts');
        }
    }

    // 2. Changed Files
    if (prMeta && prMeta.changed_files.length > 0) {
        const count = prMeta.changed_files.length;
        const rows = prMeta.changed_files
            .map(f => `| ${statusLabel(f.status)} | \`${f.path}\` |`)
            .join('\n');
        sections.push(`## Changed Files (${count} file${count !== 1 ? 's' : ''})\n| Status | File |\n|--------|------|\n${rows}`);
    }

    // 3. Test Results
    if (testResults) {
        const icon = testResults.all_passed ? '\u2705' : '\u274c';
        sections.push(`## Test Results\n${icon} ${testResults.summary}`);
    }

    // 4. PR Review Findings — only add heading when structured sections exist above,
    //    otherwise just render the raw content as-is (e.g. at the review_plan gate).
    const cleanReview = stripComposerMarkers(reviewContent);
    if (cleanReview) {
        const hasStructuredSections = prMeta !== null || testResults !== null;
        sections.push(hasStructuredSections ? `## PR Review Findings\n${cleanReview}` : cleanReview);
    }

    return sections.join('\n\n---\n\n');
}
