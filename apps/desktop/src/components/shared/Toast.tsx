import React from 'react';
import { useToastStore } from '@/hooks/useToast';
import { CheckCircle, AlertCircle, Info, AlertTriangle, X } from 'lucide-react';

export const ToastContainer: React.FC = () => {
  const { toasts, removeToast } = useToastStore();

  if (toasts.length === 0) return null;

  return (
    <div className="fixed bottom-12 left-1/2 -translate-x-1/2 z-50 flex flex-col gap-2 pointer-events-none">
      {toasts.map((toast) => {
        const Icon = {
          success: CheckCircle,
          error: AlertCircle,
          info: Info,
          warning: AlertTriangle,
        }[toast.type];

        const colors = {
          success: 'bg-green-50 text-green-800 border-green-200',
          error: 'bg-red-50 text-red-800 border-red-200',
          info: 'bg-blue-50 text-blue-800 border-blue-200',
          warning: 'bg-yellow-50 text-yellow-800 border-yellow-200',
        }[toast.type];

        const iconColors = {
          success: 'text-green-500',
          error: 'text-red-500',
          info: 'text-blue-500',
          warning: 'text-yellow-500',
        }[toast.type];

        return (
          <div
            key={toast.id}
            className={`flex items-center gap-3 px-4 py-3 rounded-lg border shadow-lg pointer-events-auto transition-all duration-300 ease-in-out ${colors}`}
            role="alert"
          >
            <Icon className={`w-5 h-5 ${iconColors}`} />
            <span className="text-sm font-medium">{toast.message}</span>
            <button
              onClick={() => removeToast(toast.id)}
              className="ml-2 p-1 rounded-md hover:bg-black/5 transition-colors"
              aria-label="关闭"
            >
              <X className="w-4 h-4 opacity-60" />
            </button>
          </div>
        );
      })}
    </div>
  );
};
