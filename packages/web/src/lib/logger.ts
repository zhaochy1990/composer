/// <reference types="vite/client" />
type LogLevel = 'debug' | 'info' | 'warn' | 'error';

const LEVELS: Record<LogLevel, number> = {
    debug: 0,
    info: 1,
    warn: 2,
    error: 3,
};

const MIN_LEVEL: LogLevel = import.meta.env.DEV ? 'debug' : 'warn';

function shouldLog(level: LogLevel): boolean {
    return LEVELS[level] >= LEVELS[MIN_LEVEL];
}

export const logger = {
    debug(msg: string, ...args: unknown[]) {
        if (shouldLog('debug')) console.debug(`[Composer] ${msg}`, ...args);
    },
    info(msg: string, ...args: unknown[]) {
        if (shouldLog('info')) console.info(`[Composer] ${msg}`, ...args);
    },
    warn(msg: string, ...args: unknown[]) {
        if (shouldLog('warn')) console.warn(`[Composer] ${msg}`, ...args);
    },
    error(msg: string, ...args: unknown[]) {
        if (shouldLog('error')) console.error(`[Composer] ${msg}`, ...args);
    },
};
