/**
 * 应用根组件
 * 配置 TanStack Query 客户端和 React Router 路由，注册所有页面路由
 */

import { useState, useEffect, type ReactElement } from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { BrowserRouter, Route, Routes } from 'react-router';
import { AppLayout } from '@/components/layout/AppLayout';
import { DashboardPage } from '@/pages/DashboardPage';
import { ClassesPage } from '@/pages/ClassesPage';
import { StudentsPage } from '@/pages/StudentsPage';
import { StudentDetailPage } from '@/pages/StudentDetailPage';
import { ImportPage } from '@/pages/ImportPage';
import { SchedulePage } from '@/pages/SchedulePage';
import { SettingsPage } from '@/pages/SettingsPage';
import { SemesterCommentsPage } from '@/pages/SemesterCommentsPage';
import { ActivityAnnouncementsPage } from '@/pages/ActivityAnnouncementsPage';
import { AssignmentGradingPage } from '@/pages/AssignmentGradingPage';
import { PracticeSheetsPage } from '@/pages/PracticeSheetsPage';
import { InitializationWizard } from '@/components/shared/InitializationWizard';
import { commands } from '@/services/commandClient';

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

export const App = (): ReactElement => {
  const [initialized, setInitialized] = useState<boolean | null>(null);

  useEffect(() => {
    checkInitialization();
  }, []);

  const checkInitialization = async () => {
    try {
      // TypeScript will pass this after bindings are regenerated
      const res = await (commands as Record<string, any>).checkInitializationStatus();
      if (res.status === 'ok') {
        setInitialized(res.data.initialized);
      } else {
        setInitialized(true);
      }
    } catch {
      setInitialized(true);
    }
  };

  if (initialized === null) {
    return (
      <div className="flex items-center justify-center h-screen bg-gray-50">
        <div className="text-gray-400 animate-pulse">加载中...</div>
      </div>
    );
  }

  return (
    <QueryClientProvider client={queryClient}>
      {!initialized && (
        <InitializationWizard onComplete={() => setInitialized(true)} />
      )}
      <BrowserRouter>
        <Routes>
          <Route element={<AppLayout />}>
            <Route path="/" element={<DashboardPage />} />
            <Route path="/classes" element={<ClassesPage />} />
            <Route path="/students" element={<StudentsPage />} />
            <Route path="/students/:id" element={<StudentDetailPage />} />
            <Route path="/import" element={<ImportPage />} />
            <Route path="/schedule" element={<SchedulePage />} />
            <Route path="/settings" element={<SettingsPage />} />
            <Route path="/semester-comments" element={<SemesterCommentsPage />} />
            <Route path="/announcements" element={<ActivityAnnouncementsPage />} />
            <Route path="/assignment-grading" element={<AssignmentGradingPage />} />
            <Route path="/practice-sheets" element={<PracticeSheetsPage />} />
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
};
