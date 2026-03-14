/**
 * 聊天服务层
 * 
 * 封装对话相关的 IPC 调用
 */

import { invoke } from '@tauri-apps/api/core';

export type Conversation = any;
export type ConversationListItem = any;
export type MessageListItem = any;
export type CreateConversationInput = any;
export type UpdateConversationInput = any;
export type ListConversationsInput = any;
export type ListConversationsResponse = any;
export type GetMessagesInput = any;

export interface ConversationFilters {
  teacherId: string;
  limit?: number;
  offset?: number;
}

export interface MessageFilters {
  conversationId: string;
  limit?: number;
  offset?: number;
}

export async function createConversation(input: CreateConversationInput): Promise<Conversation> {
  return invoke<Conversation>('create_conversation', { input });
}

export async function listConversations(filters: ConversationFilters): Promise<ListConversationsResponse> {
  const input: ListConversationsInput = {
    teacher_id: filters.teacherId,
    limit: filters.limit,
    offset: filters.offset,
  };
  return invoke<ListConversationsResponse>('list_conversations', { input });
}

export async function getConversation(id: string): Promise<Conversation> {
  return invoke<Conversation>('get_conversation', { id });
}

export async function updateConversation(input: UpdateConversationInput): Promise<Conversation> {
  return invoke<Conversation>('update_conversation', { input });
}

export async function deleteConversation(id: string): Promise<void> {
  return invoke<void>('delete_conversation', { id });
}

export async function listConversationMessages(filters: MessageFilters): Promise<MessageListItem[]> {
  const input: GetMessagesInput = {
    conversation_id: filters.conversationId,
    limit: filters.limit,
    offset: filters.offset,
  };
  return invoke<MessageListItem[]>('list_conversation_messages', { input });
}
