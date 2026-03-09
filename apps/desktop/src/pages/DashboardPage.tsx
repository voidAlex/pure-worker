/**
 * 工作台页面组件
 * 显示系统状态概览、待办任务数量和最近任务列表
 */

import React from 'react';
import { useQuery } from '@tanstack/react-query';

import { commands } from '@/services/commandClient';
import { AiWorkbench } from '@/components/dashboard/AiWorkbench';
import { isTauriRuntime } from '@/utils/runtime';

export const DashboardPage: React.FC = () => {
  const { data: healthData } = useQuery({
    queryKey: ['healthCheck'],
    queryFn: async () => {
      if (!isTauriRuntime()) {
        return { status: 'ok' as const };
      }
      const result = await commands.healthCheck();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const { data: tasksData } = useQuery({
    queryKey: ['tasks'],
    queryFn: async () => {
      if (!isTauriRuntime()) {
        return [];
      }
      const result = await commands.listTasks(null);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  return (
    <div className="space-y-6">
      <header className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold text-gray-900">工作台</h1>
          <p className="text-sm text-gray-500 mt-1">欢迎使用 PureWorker 教务助手</p>
        </div>
      </header>

      <AiWorkbench
        healthText={healthData?.status === 'ok' ? '正常运行' : '检查中...'}
        tasks={(tasksData || []).map((task) => ({
          id: task.id,
          task_type: task.task_type,
          status: task.status,
        }))}
      />
    </div>
  );
};
