import { describe, it, expect, beforeEach } from 'vitest';
import { useSessionOutputStore } from '@/stores/session-output-store';

// Reset store between tests
beforeEach(() => {
    useSessionOutputStore.setState({ outputs: {}, _seqCounters: {} });
});

describe('useSessionOutputStore', () => {
    it('has empty initial state', () => {
        const state = useSessionOutputStore.getState();
        expect(state.outputs).toEqual({});
        expect(state._seqCounters).toEqual({});
    });

    it('appends a single entry', () => {
        const { append } = useSessionOutputStore.getState();
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'hello' });
        const state = useSessionOutputStore.getState();
        expect(state.outputs['s1']).toHaveLength(1);
        expect(state.outputs['s1'][0].content).toBe('hello');
        expect(state.outputs['s1'][0].seq).toBe(1);
    });

    it('appends multiple entries with incrementing seq', () => {
        const { append } = useSessionOutputStore.getState();
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'line1' });
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'line2' });
        const state = useSessionOutputStore.getState();
        expect(state.outputs['s1']).toHaveLength(2);
        expect(state.outputs['s1'][0].seq).toBe(1);
        expect(state.outputs['s1'][1].seq).toBe(2);
    });

    it('isolates entries across sessions', () => {
        const { append } = useSessionOutputStore.getState();
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'from s1' });
        append('s2', { session_id: 's2', log_type: 'stderr', content: 'from s2' });
        const state = useSessionOutputStore.getState();
        expect(state.outputs['s1']).toHaveLength(1);
        expect(state.outputs['s2']).toHaveLength(1);
    });

    it('caps at 5000 entries (rolling window)', () => {
        const { append } = useSessionOutputStore.getState();
        for (let i = 0; i < 5010; i++) {
            append('s1', { session_id: 's1', log_type: 'stdout', content: `line ${i}` });
        }
        const state = useSessionOutputStore.getState();
        expect(state.outputs['s1']).toHaveLength(5000);
        // The oldest entries should be trimmed
        expect(state.outputs['s1'][0].content).toBe('line 10');
    });

    it('clears a single session', () => {
        const { append, clear } = useSessionOutputStore.getState();
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'data' });
        append('s2', { session_id: 's2', log_type: 'stdout', content: 'data' });
        clear('s1');
        const state = useSessionOutputStore.getState();
        expect(state.outputs['s1']).toBeUndefined();
        expect(state.outputs['s2']).toHaveLength(1);
    });

    it('clear does not affect other sessions', () => {
        const { append, clear } = useSessionOutputStore.getState();
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'a' });
        append('s2', { session_id: 's2', log_type: 'stdout', content: 'b' });
        clear('s1');
        const state = useSessionOutputStore.getState();
        expect(state._seqCounters['s1']).toBeUndefined();
        expect(state._seqCounters['s2']).toBe(1);
    });

    it('seq counter increments independently per session', () => {
        const { append } = useSessionOutputStore.getState();
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'a' });
        append('s1', { session_id: 's1', log_type: 'stdout', content: 'b' });
        append('s2', { session_id: 's2', log_type: 'stdout', content: 'c' });
        const state = useSessionOutputStore.getState();
        expect(state._seqCounters['s1']).toBe(2);
        expect(state._seqCounters['s2']).toBe(1);
    });
});
