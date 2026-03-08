/**
 * 底部状态栏组件
 * 显示系统状态、本地优先模式和版本号
 */

import React from 'react';
import { Activity, CheckCircle2 } from 'lucide-react';

export const StatusBar: React.FC = () => {
  return (
    <footer className="h-8 bg-gray-50 border-t border-gray-200 flex items-center justify-between px-4 text-xs text-gray-500 shrink-0">
      <div className="flex items-center gap-4">
        <span className="font-medium text-gray-700">PureWorker</span>
        <span className="flex items-center gap-1.5">
          <CheckCircle2 className="w-3.5 h-3.5 text-green-500" />
          系统正常
        </span>
      </div>
      
      <div className="flex items-center gap-4">
        <span className="flex items-center gap-1.5">
          <Activity className="w-3.5 h-3.5 text-brand-500" />
          本地优先
        </span>
        <span>v1.0.0</span>
      </div>
    </footer>
  );
};
