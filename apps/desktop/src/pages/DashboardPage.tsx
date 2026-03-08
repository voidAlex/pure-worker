/**
 * 工作台页面组件
 * 显示系统状态概览、待办任务数量和最近任务列表
 */

import React from 'react';
import { useQuery } from '@tanstack/react-query';
import { commands } from '@/bindings';
import { EmptyState } from '@/components/shared/EmptyState';
import { Activity, Users, CalendarDays, CheckCircle2 } from 'lucide-react';

export const DashboardPage: React.FC = () => {
  const { data: healthData } = useQuery({
    queryKey: ['healthCheck'],
    queryFn: async () => {
      const result = await commands.healthCheck();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const { data: tasksData } = useQuery({
    queryKey: ['tasks'],
    queryFn: async () => {
      const result = await commands.listTasks();
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

      <div className="grid grid-cols-1 md:grid-cols-3 gap-6">
        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 flex items-center gap-4">
          <div className="p-3 bg-blue-50 text-blue-600 rounded-lg">
            <Activity className="w-6 h-6" />
          </div>
          <div>
            <p className="text-sm font-medium text-gray-500">系统状态</p>
            <p className="text-lg font-semibold text-gray-900 flex items-center gap-2">
              {healthData?.status === 'ok' ? '正常运行' : '检查中...'}
              {healthData?.status === 'ok' && <CheckCircle2 className="w-4 h-4 text-green-500" />}
            </p>
          </div>
        </div>

        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 flex items-center gap-4">
          <div className="p-3 bg-purple-50 text-purple-600 rounded-lg">
            <Users className="w-6 h-6" />
          </div>
          <div>
            <p className="text-sm font-medium text-gray-500">待办任务</p>
            <p className="text-lg font-semibold text-gray-900">
              {tasksData?.length || 0} 项
            </p>
          </div>
        </div>

        <div className="bg-white p-6 rounded-xl shadow-sm border border-gray-100 flex items-center gap-4">
          <div className="p-3 bg-orange-50 text-orange-600 rounded-lg">
            <CalendarDays className="w-6 h-6" />
          </div>
          <div>
            <p className="text-sm font-medium text-gray-500">今日日程</p>
            <p className="text-lg font-semibold text-gray-900">0 项</p>
          </div>
        </div>
      </div>

      <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div className="px-6 py-4 border-b border-gray-100">
          <h2 className="text-lg font-semibold text-gray-900">最近任务</h2>
        </div>
        <div className="p-6">
          {tasksData && tasksData.length > 0 ? (
            <ul className="space-y-4">
              {tasksData.map((task) => (
                <li key={task.id} className="flex items-center justify-between p-4 bg-gray-50 rounded-lg border border-gray-100">
                  <div className="flex items-center gap-3">
                    <div className="w-2 h-2 rounded-full bg-brand-500"></div>
                    <span className="font-medium text-gray-900">{task.task_type}</span>
                  </div>
                  <span className="text-sm text-gray-500">{task.status}</span>
                </li>
              ))}
            </ul>
          ) : (
            <EmptyState 
              title="暂无任务" 
              description="当前没有正在进行或待处理的任务" 
            />
          )}
        </div>
      </div>
    </div>
  );
};
