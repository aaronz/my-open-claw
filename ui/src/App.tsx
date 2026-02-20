import React, { useState, useEffect, useRef } from 'react';
import { Send, User, Bot, Loader2, Maximize2, Layout } from 'lucide-react';

interface Message {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const [canvas, setCanvas] = useState<{title?: string, content: string, language?: string} | null>(null);
  const ws = useRef<WebSocket | null>(null);
  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host === 'localhost:5173' ? 'localhost:18789' : window.location.host;
    ws.current = new WebSocket(`${protocol}//${host}/ws`);

    ws.current.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.type === 'agent_thinking') {
        setThinking(true);
      } else if (msg.type === 'agent_response') {
        setThinking(false);
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last && last.role === 'assistant') {
            return [...prev.slice(0, -1), { ...last, content: last.content + msg.content }];
          }
          return [...prev, { role: 'assistant', content: msg.content }];
        });
      } else if (msg.type === 'canvas_update') {
        setCanvas({ title: msg.title, content: msg.content, language: msg.language });
      }
    };

    return () => ws.current?.close();
  }, []);

  useEffect(() => {
    chatEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, thinking]);

  const sendMessage = () => {
    if (!input.trim() || !ws.current) return;
    setMessages(prev => [...prev, { role: 'user', content: input }]);
    ws.current.send(JSON.stringify({
      type: 'send_message',
      content: input,
      channel: 'webchat',
      peer_id: 'browser-user'
    }));
    setInput('');
  };

  return (
    <div className="flex h-screen bg-gray-50 overflow-hidden">
      <div className={`flex flex-col ${canvas ? 'w-1/3' : 'w-full'} border-r bg-white transition-all duration-300`}>
        <header className="p-4 border-bottom flex items-center gap-2">
          <div className="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center text-white font-bold">OC</div>
          <h1 className="font-bold text-xl">OpenClaw</h1>
        </header>

        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {messages.map((m, i) => (
            <div key={i} className={`flex ${m.role === 'user' ? 'justify-end' : 'justify-start'}`}>
              <div className={`max-w-[85%] rounded-2xl px-4 py-2 ${
                m.role === 'user' ? 'bg-blue-600 text-white rounded-br-none' : 'bg-gray-100 text-gray-800 rounded-bl-none'
              }`}>
                {m.content}
              </div>
            </div>
          ))}
          {thinking && (
            <div className="flex justify-start">
              <div className="bg-gray-100 text-gray-400 rounded-2xl px-4 py-2 flex items-center gap-2 italic text-sm">
                <Loader2 className="w-3 h-3 animate-spin" />
                Thinking...
              </div>
            </div>
          )}
          <div ref={chatEndRef} />
        </div>

        <div className="p-4 border-t">
          <div className="relative flex items-center">
            <input
              type="text"
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && sendMessage()}
              placeholder="Message OpenClaw..."
              className="w-full bg-gray-100 border-none rounded-full py-3 px-6 pr-12 focus:ring-2 focus:ring-blue-500 outline-none transition-all"
            />
            <button 
              onClick={sendMessage}
              className="absolute right-2 p-2 bg-blue-600 text-white rounded-full hover:bg-blue-700 transition-colors"
            >
              <Send className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {canvas && (
        <div className="flex-1 flex flex-col bg-white h-full animate-in slide-in-from-right duration-300">
          <header className="p-4 border-b flex justify-between items-center bg-gray-50">
            <div className="flex items-center gap-2">
              <Layout className="w-4 h-4 text-gray-500" />
              <span className="font-semibold">{canvas.title || 'Artifact'}</span>
              {canvas.language && <span className="text-xs bg-gray-200 px-2 py-0.5 rounded text-gray-600 uppercase font-mono">{canvas.language}</span>}
            </div>
            <button onClick={() => setCanvas(null)} className="text-gray-400 hover:text-gray-600"><Maximize2 className="w-4 h-4" /></button>
          </header>
          <div className="flex-1 overflow-auto p-8">
            <pre className="font-mono text-sm bg-gray-50 p-6 rounded-xl border border-gray-100 whitespace-pre-wrap">
              {canvas.content}
            </pre>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;
