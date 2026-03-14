import { commands as tauriCommands, type AppError, type Result } from '@/bindings';
import { isTauriRuntime } from '@/utils/runtime';

const WEB_MODE_ERROR: AppError = {
  ExternalService: '当前为 Web 预览环境，该操作需要桌面端（Tauri）支持。',
};

const wrapWebUnsupported = async <T>(): Promise<Result<T, AppError>> => ({
  status: 'error',
  error: WEB_MODE_ERROR,
});

type CommandName = keyof typeof tauriCommands;

type CommandFunction<Name extends CommandName> = (typeof tauriCommands)[Name];

type WrapFunction<Fn> = Fn extends (...args: infer A) => Promise<infer R>
  ? (...args: A) => Promise<R>
  : never;

type WrappedCommands = {
  [Name in CommandName]: WrapFunction<CommandFunction<Name>>;
};

const makeWrappedCommands = (): WrappedCommands => {
  return new Proxy({} as WrappedCommands, {
    get: (_target, prop: string) => {
      const commandName = prop as CommandName;

      if (!(commandName in tauriCommands)) {
        return undefined;
      }

      return async (...args: unknown[]) => {
        if (!isTauriRuntime()) {
          return wrapWebUnsupported();
        }

        const command = tauriCommands[commandName] as (...input: unknown[]) => Promise<unknown>;
        return command(...args);
      };
    },
  });
};

export const commands = makeWrappedCommands();

export * from '@/bindings';
