import React, { useState, useEffect, useRef } from 'react';
import { Send, User, Bot, Loader2, Maximize2, Layout } from 'lucide-react';

interface Message {
  id?: string;
  role: 'user' | 'assistant' | 'system';
  content: string;
  images?: string[];
}

function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const [canvas, setCanvas] = useState<{title?: string, content: string, language?: string} | null>(null);
  const [toolStatus, setToolStatus] = useState<string | null>(null);
  const [connected, setConnected] = useState(false);
  const ws = useRef<WebSocket | null>(null);
  const chatEndRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const host = window.location.host === 'localhost:5173' ? 'localhost:18789' : window.location.host;
    ws.current = new WebSocket(`${protocol}//${host}/ws`);

    ws.current.onopen = () => setConnected(true);
    ws.current.onclose = () => setConnected(false);

    ws.current.onmessage = (event) => {
      const msg = JSON.parse(event.data);
      if (msg.type === 'agent_thinking') {
        setThinking(true);
        setToolStatus(null);
      } else if (msg.type === 'agent_response') {
        setThinking(false);
        setToolStatus(null);
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last && last.role === 'assistant') {
            return [...prev.slice(0, -1), { ...last, content: last.content + msg.content }];
          }
          return [...prev, { role: 'assistant', content: msg.content }];
        });
      } else if (msg.type === 'canvas_update') {
        setCanvas({ title: msg.title, content: msg.content, language: msg.language });
      } else if (msg.type === 'new_message') {
        if (msg.message.role === 'assistant' || msg.message.role === 'system') {
           setMessages(prev => [...prev, msg.message]);
        }
      } else if (msg.type === 'presence_update') {
        console.log("Presence:", msg.status);
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
        <header className="p-4 border-b flex items-center justify-between">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 bg-blue-600 rounded-lg flex items-center justify-center text-white font-bold">OC</div>
            <h1 className="font-bold text-xl">OpenClaw</h1>
          </div>
          <div className={`w-2 h-2 rounded-full ${connected ? 'bg-green-500' : 'bg-red-500'} animate-pulse`} title={connected ? 'Connected' : 'Disconnected'} />
        </header>

        <div className="flex-1 overflow-y-auto p-4 space-y-4">
          {messages.map((m, i) => (
            <div key={i} className={`flex ${m.role === 'user' ? 'justify-end' : (m.role === 'system' ? 'justify-center' : 'justify-start')}`}>
              <div className={`max-w-[85%] rounded-2xl px-4 py-2 ${
                m.role === 'user' 
                  ? 'bg-blue-600 text-white rounded-br-none shadow-sm' 
                  : (m.role === 'system' ? 'bg-amber-50 text-amber-700 text-xs font-medium uppercase tracking-wider rounded-lg border border-amber-100 px-3 py-1' : 'bg-gray-100 text-gray-800 rounded-bl-none shadow-xs')
              }`}>
                {m.images && m.images.length > 0 && (
                  <div className="grid grid-cols-2 gap-2 mb-2">
                    {m.images.map((img, idx) => (
                      <img key={idx} src={`data:image/jpeg;base64,${img}`} className="rounded-lg w-full h-auto" alt="attached" />
                    ))}
                  </div>
                )}
                <div className="whitespace-pre-wrap">{m.content}</div>
              </div>
            </div>
          ))}
          {thinking && (
            <div className="flex justify-start">
              <div className="bg-gray-100 text-gray-400 rounded-2xl px-4 py-2 flex flex-col gap-2 italic text-sm border border-gray-200">
                <div className="flex items-center gap-2">
                  <Loader2 className="w-3 h-3 animate-spin" />
                  {toolStatus || 'OpenClaw is thinking...'}
                </div>
                <div className="flex gap-1 h-2 items-center">
                  {[1,2,3,4,5].map(i => (
                    <div key={i} className="w-1.5 bg-blue-300 rounded-full animate-bounce" style={{ height: '100%', animationDelay: `${i * 0.15}s` }} />
                  ))}
                </div>
              </div>
            </div>
          )}
          <div ref={chatEndRef} />
        </div>

        <div className="p-4 border-t bg-gray-50">
          <div className="relative flex items-center max-w-4xl mx-auto w-full">
            <input
              type="text"
              value={input}
              onChange={e => setInput(e.target.value)}
              onKeyDown={e => e.key === 'Enter' && sendMessage()}
              placeholder="Message OpenClaw..."
              className="w-full bg-white border border-gray-200 rounded-full py-3 px-6 pr-12 focus:ring-2 focus:ring-blue-500 shadow-sm outline-none transition-all"
            />
            <button 
              onClick={sendMessage}
              disabled={!connected}
              className={`absolute right-2 p-2 rounded-full transition-colors ${connected ? 'bg-blue-600 text-white hover:bg-blue-700 shadow-md' : 'bg-gray-300 text-gray-500 cursor-not-allowed'}`}
            >
              <Send className="w-4 h-4" />
            </button>
          </div>
        </div>
      </div>

      {canvas && (
        <div className="flex-1 flex flex-col bg-white h-full animate-in slide-in-from-right duration-300 shadow-2xl z-10 border-l border-gray-200">
          <header className="p-4 border-b flex justify-between items-center bg-gray-50/80 backdrop-blur-sm">
            <div className="flex items-center gap-2">
              <Layout className="w-4 h-4 text-gray-500" />
              <span className="font-semibold text-gray-700">{canvas.title || 'Artifact'}</span>
              {canvas.language && <span className="text-[10px] bg-blue-100 px-2 py-0.5 rounded-full text-blue-600 uppercase font-bold tracking-tighter">{canvas.language}</span>}
            </div>
            <button onClick={() => setCanvas(null)} className="p-1.5 hover:bg-gray-200 rounded-lg text-gray-400 transition-colors"><Maximize2 className="w-4 h-4" /></button>
          </header>
          <div className="flex-1 overflow-auto bg-gray-50/30">
             <div className="max-w-4xl mx-auto p-8 h-full">
                <div className="bg-white rounded-2xl shadow-xl border border-gray-100 h-full flex flex-col overflow-hidden">
                  <div className="p-1 bg-gray-50 border-b flex gap-1.5 px-4">
                    <div className="w-3 h-3 rounded-full bg-red-400" />
                    <div className="w-3 h-3 rounded-full bg-amber-400" />
                    <div className="w-3 h-3 rounded-full bg-green-400" />
                  </div>
                  <pre className="flex-1 font-mono text-sm p-6 overflow-auto whitespace-pre-wrap selection:bg-blue-100">
                    {canvas.content}
                  </pre>
                </div>
             </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default App;


function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [thinking, setThinking] = useState(false);
  const [canvas, setCanvas] = useState<{title?: string, content: string, language?: string} | null>(null);
  const [toolStatus, setToolStatus] = useState<string | null>(null);
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
        setToolStatus(null);
      } else if (msg.type === 'agent_response') {
        setThinking(false);
        setToolStatus(null);
        setMessages(prev => {
          const last = prev[prev.length - 1];
          if (last && last.role === 'assistant') {
            return [...prev.slice(0, -1), { ...last, content: last.content + msg.content }];
          }
          return [...prev, { role: 'assistant', content: msg.content }];
        });
      } else if (msg.type === 'canvas_update') {
        setCanvas({ title: msg.title, content: msg.content, language: msg.language });
      } else if (msg.type === 'new_message') {
        if (msg.message.role === 'assistant' || msg.message.role === 'system') {
           setMessages(prev => [...prev, msg.message]);
        }
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
            <div key={i} className={`flex ${m.role === 'user' ? 'justify-end' : (m.role === 'system' ? 'justify-center' : 'justify-start')}`}>
              <div className={`max-w-[85%] rounded-2xl px-4 py-2 ${
                m.role === 'user' 
                  ? 'bg-blue-600 text-white rounded-br-none' 
                  : (m.role === 'system' ? 'bg-amber-50 text-amber-700 text-xs font-medium uppercase tracking-wider rounded-lg border border-amber-100' : 'bg-gray-100 text-gray-800 rounded-bl-none')
              }`}>
                {m.content}
              </div>
            </div>
          ))}
          {thinking && (
            <div className="flex justify-start">
              <div className="bg-gray-100 text-gray-400 rounded-2xl px-4 py-2 flex flex-col gap-2 italic text-sm">
                <div className="flex items-center gap-2">
                  <Loader2 className="w-3 h-3 animate-spin" />
                  {toolStatus || 'OpenClaw is thinking...'}
                </div>
                <div className="flex gap-0.5 h-3 items-center">
                  {[1,2,3,4,5].map(i => (
                    <div key={i} className="w-1 bg-blue-400 rounded-full animate-pulse" style={{ height: `${20 + Math.random() * 80}%`, animationDelay: `${i * 0.1}s` }} />
                  ))}
                </div>
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
