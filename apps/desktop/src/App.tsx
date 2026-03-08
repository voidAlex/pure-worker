/**
 * 应用根组件
 * 配置 TanStack Query 客户端和 React Router 路由
 */

import { type ReactElement } from 'react';
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

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      refetchOnWindowFocus: false,
      retry: 1,
    },
  },
});

export const App = (): ReactElement => {
  return (
    <QueryClientProvider client={queryClient}>
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
          </Route>
        </Routes>
      </BrowserRouter>
    </QueryClientProvider>
  );
};
