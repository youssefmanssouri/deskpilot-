import type { ErrorCode, IpcErrorDto } from './types';

/**
 * Structured client-side error class representing a failure from the Rust IPC boundary.
 */
export class IpcError extends Error {
  public readonly code: ErrorCode | string;
  public readonly details?: string;

  constructor(dto: IpcErrorDto) {
    super(dto.message);
    this.name = 'IpcError';
    this.code = dto.code;
    this.details = dto.details;
  }

  public isInvalidInput(): boolean {
    return this.code === 'INVALID_INPUT';
  }

  public isNotFound(): boolean {
    return this.code === 'NOT_FOUND';
  }

  public isPermissionDenied(): boolean {
    return this.code === 'PERMISSION_DENIED';
  }

  public isUnavailable(): boolean {
    return this.code === 'UNAVAILABLE';
  }

  public isOperationFailed(): boolean {
    return this.code === 'OPERATION_FAILED';
  }

  public isInternalError(): boolean {
    return this.code === 'INTERNAL_ERROR';
  }

  /**
   * Safely constructs an IpcError from an arbitrary thrown error or rejected promise.
   */
  public static fromResponse(error: unknown): IpcError {
    if (error instanceof IpcError) {
      return error;
    }

    if (error && typeof error === 'object') {
      const candidate = error as Partial<IpcErrorDto>;
      if (typeof candidate.code === 'string' && typeof candidate.message === 'string') {
        return new IpcError({
          code: candidate.code,
          message: candidate.message,
          details: candidate.details,
        });
      }
    }

    if (typeof error === 'string') {
      return new IpcError({
        code: 'INTERNAL_ERROR',
        message: error,
      });
    }

    return new IpcError({
      code: 'INTERNAL_ERROR',
      message: 'An unknown IPC communication error occurred.',
    });
  }
}

