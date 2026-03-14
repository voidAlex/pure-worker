/**
 * 侧边栏导航组件
 * 可折叠的左侧导航菜单，包含工作台、班级管理、学生档案、数据导入、课表日程、作业批改、错题练习入口
 */

import React from 'react';
import { NavLink } from 'react-router';
import {
  LayoutDashboard,
  Users,
  GraduationCap,
  UploadCloud,
  CalendarDays,
  ChevronLeft,
  ChevronRight,
  Settings,
  FileText,
  Megaphone,
  ClipboardCheck,
  BookOpen,
} from 'lucide-react';

interface SidebarProps {
  isCollapsed: boolean;
  toggleCollapse: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ isCollapsed, toggleCollapse }) => {
  const navItems = [
    { path: '/', icon: LayoutDashboard, label: '工作台' },
    { path: '/classes', icon: Users, label: '班级管理' },
    { path: '/students', icon: GraduationCap, label: '学生档案' },
    { path: '/import', icon: UploadCloud, label: '数据导入' },
    { path: '/schedule', icon: CalendarDays, label: '课表日程' },
    { path: '/semester-comments', icon: FileText, label: '期末评语' },
    { path: '/announcements', icon: Megaphone, label: '班会活动' },
    { path: '/assignment-grading', icon: ClipboardCheck, label: '作业批改' },
    { path: '/practice-sheets', icon: BookOpen, label: '错题练习' },
    { path: '/settings', icon: Settings, label: 'AI 配置' },
  ];

  return (
    <aside
      className={`bg-white border-r border-gray-200 flex flex-col transition-all duration-300 ease-in-out shrink-0 ${
        isCollapsed ? 'w-16' : 'w-[220px]'
      }`}
    >
      <div className="h-14 flex items-center justify-between px-4 border-b border-gray-100">
        {!isCollapsed && <span className="font-bold text-brand-700 truncate">PureWorker</span>}
        <button
          onClick={toggleCollapse}
          className={`p-1.5 rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-100 transition-colors ${
            isCollapsed ? 'mx-auto' : ''
          }`}
          aria-label={isCollapsed ? '展开侧边栏' : '折叠侧边栏'}
        >
          {isCollapsed ? <ChevronRight className="w-4 h-4" /> : <ChevronLeft className="w-4 h-4" />}
        </button>
      </div>

      <nav className="flex-1 py-4 px-2 space-y-1 overflow-y-auto">
        {navItems.map((item) => (
          <NavLink
            key={item.path}
            to={item.path}
            className={({ isActive }) =>
              `flex items-center px-3 py-2.5 rounded-lg transition-colors group ${
                isActive
                  ? 'bg-brand-50 text-brand-700 font-medium'
                  : 'text-gray-600 hover:bg-gray-50 hover:text-gray-900'
              } ${isCollapsed ? 'justify-center' : ''}`
            }
            title={isCollapsed ? item.label : undefined}
          >
            <item.icon className={`w-5 h-5 flex-shrink-0 ${isCollapsed ? '' : 'mr-3'}`} />
            {!isCollapsed && <span className="truncate">{item.label}</span>}
          </NavLink>
        ))}
      </nav>
    </aside>
  );
};
