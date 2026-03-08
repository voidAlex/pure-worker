import React, { useState } from 'react';
import { useQuery, useMutation } from '@tanstack/react-query';
import { commands, ImportStudentsInput, ImportDuplicateStrategy, ImportStudentsResult } from '@/bindings';
import { useToast } from '@/hooks/useToast';
import { UploadCloud, FileSpreadsheet, AlertCircle, CheckCircle2, Info } from 'lucide-react';

export const ImportPage: React.FC = () => {
  const { success, error } = useToast();
  const [formData, setFormData] = useState<ImportStudentsInput>({
    file_path: '',
    class_id: '',
    duplicate_strategy: 'Skip',
  });
  const [importResult, setImportResult] = useState<ImportStudentsResult | null>(null);

  const { data: classes } = useQuery({
    queryKey: ['classrooms'],
    queryFn: async () => {
      const result = await commands.listClassrooms();
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
  });

  const importMutation = useMutation({
    mutationFn: async (input: ImportStudentsInput) => {
      const result = await commands.importStudents(input);
      if (result.status === 'error') throw new Error(JSON.stringify(result.error));
      return result.data;
    },
    onSuccess: (data) => {
      setImportResult(data);
      if (data.error_count === 0) {
        success(`导入成功: 新增 ${data.created_count} 条，更新 ${data.updated_count} 条`);
      } else {
        error(`导入完成，但有 ${data.error_count} 条错误`);
      }
    },
    onError: (err) => error(`导入失败: ${err.message}`),
  });

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    setImportResult(null);
    importMutation.mutate(formData);
  };

  return (
    <div className="space-y-6 max-w-4xl mx-auto">
      <header>
        <h1 className="text-2xl font-bold text-gray-900">数据导入</h1>
        <p className="text-sm text-gray-500 mt-1">从 Excel 文件批量导入学生数据</p>
      </header>

      <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden">
        <div className="p-6 border-b border-gray-100 bg-gray-50/50">
          <div className="flex items-start gap-4">
            <div className="p-3 bg-blue-50 text-blue-600 rounded-lg shrink-0">
              <Info className="w-6 h-6" />
            </div>
            <div>
              <h3 className="text-sm font-medium text-gray-900">导入说明</h3>
              <ul className="mt-2 text-sm text-gray-600 space-y-1 list-disc list-inside">
                <li>支持 .xlsx 和 .csv 格式文件</li>
                <li>必须包含列：学号、姓名</li>
                <li>可选列：性别（男/女）</li>
                <li>请确保文件未被其他程序占用</li>
              </ul>
            </div>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="p-6 space-y-6">
          <div className="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div className="space-y-4">
              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">目标班级</label>
                <select
                  required
                  value={formData.class_id}
                  onChange={(e) => setFormData({ ...formData, class_id: e.target.value })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                >
                  <option value="" disabled>请选择导入班级</option>
                  {classes?.map((cls) => (
                    <option key={cls.id} value={cls.id}>
                      {cls.grade} {cls.class_name}
                    </option>
                  ))}
                </select>
              </div>

              <div>
                <label className="block text-sm font-medium text-gray-700 mb-1">重复处理策略</label>
                <select
                  required
                  value={formData.duplicate_strategy}
                  onChange={(e) => setFormData({ ...formData, duplicate_strategy: e.target.value as ImportDuplicateStrategy })}
                  className="w-full px-3 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow"
                >
                  <option value="Skip">跳过 (保留原有数据)</option>
                  <option value="Update">更新 (覆盖原有数据)</option>
                  <option value="Add">新增 (允许重复学号)</option>
                </select>
              </div>
            </div>

            <div>
              <label className="block text-sm font-medium text-gray-700 mb-1">文件路径</label>
              <div className="relative">
                <input
                  type="text"
                  required
                  value={formData.file_path}
                  onChange={(e) => setFormData({ ...formData, file_path: e.target.value })}
                  placeholder="例如：C:\Users\Admin\Desktop\students.xlsx"
                  className="w-full pl-10 pr-4 py-2 border border-gray-300 rounded-lg focus:ring-2 focus:ring-brand-500 focus:border-brand-500 outline-none transition-shadow font-mono text-sm"
                />
                <FileSpreadsheet className="w-5 h-5 text-gray-400 absolute left-3 top-1/2 -translate-y-1/2" />
              </div>
              <p className="mt-2 text-xs text-gray-500">
                请输入或粘贴本地 Excel 文件的绝对路径
              </p>
            </div>
          </div>

          <div className="pt-4 border-t border-gray-100 flex justify-end">
            <button
              type="submit"
              disabled={importMutation.isPending || !formData.class_id || !formData.file_path}
              className="flex items-center gap-2 px-6 py-2.5 bg-brand-600 text-white rounded-lg hover:bg-brand-700 transition-colors shadow-sm font-medium disabled:opacity-50 disabled:cursor-not-allowed"
            >
              {importMutation.isPending ? (
                <>
                  <div className="w-4 h-4 border-2 border-white/30 border-t-white rounded-full animate-spin" />
                  导入中...
                </>
              ) : (
                <>
                  <UploadCloud className="w-4 h-4" />
                  开始导入
                </>
              )}
            </button>
          </div>
        </form>
      </div>

      {importResult && (
        <div className="bg-white rounded-xl shadow-sm border border-gray-100 overflow-hidden animate-in fade-in slide-in-from-bottom-4 duration-500">
          <div className="p-6 border-b border-gray-100 flex items-center justify-between">
            <h3 className="text-lg font-semibold text-gray-900">导入结果</h3>
            <div className="flex items-center gap-2 text-sm font-medium">
              <span className="text-gray-500">总计: {importResult.total_rows}</span>
              <span className="text-green-600 bg-green-50 px-2 py-0.5 rounded">新增: {importResult.created_count}</span>
              <span className="text-blue-600 bg-blue-50 px-2 py-0.5 rounded">更新: {importResult.updated_count}</span>
              <span className="text-gray-600 bg-gray-100 px-2 py-0.5 rounded">跳过: {importResult.skipped_count}</span>
              <span className="text-red-600 bg-red-50 px-2 py-0.5 rounded">错误: {importResult.error_count}</span>
            </div>
          </div>

          {importResult.errors.length > 0 ? (
            <div className="overflow-x-auto">
              <table className="w-full text-left border-collapse">
                <thead>
                  <tr className="bg-red-50/50 border-b border-red-100 text-sm font-medium text-red-800">
                    <th className="px-6 py-3">行号</th>
                    <th className="px-6 py-3">字段</th>
                    <th className="px-6 py-3">错误原因</th>
                    <th className="px-6 py-3">建议</th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-gray-100">
                  {importResult.errors.map((err, idx) => (
                    <tr key={idx} className="hover:bg-gray-50 transition-colors text-sm">
                      <td className="px-6 py-3 text-gray-500 font-mono">第 {err.row_number} 行</td>
                      <td className="px-6 py-3 font-medium text-gray-900">{err.field}</td>
                      <td className="px-6 py-3 text-red-600 flex items-center gap-1.5">
                        <AlertCircle className="w-4 h-4" />
                        {err.reason}
                      </td>
                      <td className="px-6 py-3 text-gray-600">{err.suggestion}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          ) : (
            <div className="p-12 text-center flex flex-col items-center justify-center">
              <div className="w-16 h-16 bg-green-50 text-green-500 rounded-full flex items-center justify-center mb-4">
                <CheckCircle2 className="w-8 h-8" />
              </div>
              <h4 className="text-lg font-medium text-gray-900 mb-1">全部导入成功</h4>
              <p className="text-sm text-gray-500">没有发现任何错误记录</p>
            </div>
          )}
        </div>
      )}
    </div>
  );
};
