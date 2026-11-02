import { useCallback, useEffect, useMemo, useState } from "react";
import {
  AlertTriangle,
  Check,
  Clock3,
  Gamepad2,
  Loader2,
  MessageSquare,
  RefreshCw,
  Search,
  Send,
  UserPlus,
  Users,
  X,
} from "lucide-react";
import { api, SocialConversation, SocialFriend, SocialMessage, SocialSnapshot } from "../lib/api";
import { cn } from "../lib/format";
import { usePoll } from "../lib/usePoll";

function relativeTime(value: number | null) {
  if (!value) return "a while ago";
  const diff = Date.now() - value;
  const minutes = Math.floor(diff / 60000);
  if (minutes < 1) return "just now";
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

function messageTime(value: string) {
  const number = Number(value);
  const date = new Date(Number.isFinite(number) ? number : value);
  if (Number.isNaN(date.getTime())) return "";
  return date.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" });
}

function friendName(friend: SocialFriend) {
  const name = (friend.gameName || "Unknown").trim();
  const tag = (friend.tagLine || "").trim();
  return `${name}${tag ? `#${tag}` : ""}`;
}

function presenceLabel(friend: SocialFriend) {
  if (friend.presence.queue) {
    return `${friend.presence.queue}${friend.presence.map ? ` · ${friend.presence.map}` : ""}`;
  }
  if (friend.presence.state === "mobile") return "On mobile";
  if (friend.presence.product === "valorant") return "VALORANT";
  if (friend.presence.product === "league_of_legends") return "League of Legends";
  return friend.presence.product || friend.presence.state;
}

export default function Social() {
  const [data, setData] = useState<SocialSnapshot | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<{ cid: string; name: string; kind: string } | null>(
    null,
  );
  const [messages, setMessages] = useState<SocialMessage[]>([]);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState<string | null>(null);
  const [requestName, setRequestName] = useState("");
  const [requestTag, setRequestTag] = useState("");
  const [notice, setNotice] = useState<string | null>(null);

  const load = useCallback(async () => {
    try {
      const snapshot = await api.socialSnapshot();
      setData(snapshot);
      setError(null);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  usePoll(load, 15000);

  useEffect(() => {
    void load();
  }, [load]);

  const loadMessages = useCallback(async (cid: string) => {
    try {
      const result = await api.socialMessages(cid);
      setMessages(result.messages ?? []);
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  }, []);

  useEffect(() => {
    if (!selected) {
      setMessages([]);
      return;
    }
    void loadMessages(selected.cid);
    const timer = window.setInterval(() => {
      if (!document.hidden) void loadMessages(selected.cid);
    }, 10000);
    return () => window.clearInterval(timer);
  }, [selected, loadMessages]);

  const friends = useMemo(() => {
    const needle = search.trim().toLowerCase();
    return (data?.friends ?? [])
      .filter((friend) => {
        if (!needle) return true;
        return friendName(friend).toLowerCase().includes(needle);
      })
      .sort((a, b) => Number(b.online) - Number(a.online) || friendName(a).localeCompare(friendName(b)));
  }, [data, search]);

  const onlineCount = data?.friends.filter((friend) => friend.online).length ?? 0;

  function conversationFor(friend: SocialFriend): SocialConversation | undefined {
    return data?.conversations.find(
      (conversation) =>
        conversation.cid.startsWith(friend.puuid) ||
        conversation.participants.some((participant) => participant.puuid === friend.puuid),
    );
  }

  function openFriend(friend: SocialFriend) {
    const conversation = conversationFor(friend);
    const cid = conversation?.cid ?? friend.pid?.split("/")[0] ?? friend.puuid;
    setSelected({
      cid,
      name: friendName(friend),
      kind: conversation?.type ?? "chat",
    });
    setNotice(null);
  }

  function openConversation(conversation: SocialConversation) {
    const others = conversation.participants.filter(
      (participant) => participant.puuid !== data?.puuid,
    );
    const label = others
      .map((participant) =>
        `${participant.gameName}${participant.tagLine ? `#${participant.tagLine}` : ""}`,
      )
      .filter(Boolean)
      .join(", ");
    setSelected({
      cid: conversation.cid,
      name: label || (conversation.type === "groupchat" ? "Group chat" : "Riot chat"),
      kind: conversation.type,
    });
  }

  async function send() {
    if (!selected || !draft.trim()) return;
    setBusy("send");
    try {
      await api.socialSend(selected.cid, draft.trim(), selected.kind);
      setDraft("");
      setNotice("Message sent.");
      await loadMessages(selected.cid);
      await load();
    } catch (cause) {
      setNotice(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function addFriend() {
    if (!requestName.trim() || !requestTag.trim()) return;
    setBusy("request");
    try {
      await api.socialSendRequest(requestName.trim(), requestTag.trim());
      setRequestName("");
      setRequestTag("");
      setNotice("Friend request sent.");
      await load();
    } catch (cause) {
      setNotice(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  async function removeRequest(puuid: string) {
    setBusy(puuid);
    try {
      await api.socialRemoveRequest(puuid);
      await load();
    } catch (cause) {
      setNotice(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(null);
    }
  }

  return (
    <div className="page social-page">
      <header className="page-head">
        <div>
          <span className="eyebrow">
            <MessageSquare size={12} /> social
          </span>
          <h1>Friends & messages.</h1>
          <p>
            Your live friends list, presence and Riot chat in one place, including the
            pinned Valotrack Active command friend.
          </p>
        </div>
        <div className="head-status">
          {data?.account && (
            <span className="account-badge">
              <span className="dot" />
              {data.account}
            </span>
          )}
          <button className="button small" onClick={() => load()} disabled={busy !== null}>
            <RefreshCw size={13} /> Refresh
          </button>
        </div>
      </header>

      {error && (
        <div className="error-banner">
          <AlertTriangle size={16} />
          <div>
            <strong>Social unavailable</strong>
            <span>{error}</span>
          </div>
          <button className="button small" onClick={() => load()}>
            Retry
          </button>
        </div>
      )}

      {notice && (
        <div className="action-result ok">
          <Check size={15} />
          <span>{notice}</span>
          <button className="icon-button" onClick={() => setNotice(null)}>
            <X size={14} />
          </button>
        </div>
      )}

      <div className="social-layout">
        <section className="panel social-friends">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Friends</span>
              <h2>
                {onlineCount} online <em>of {data?.friends.length ?? 0}</em>
              </h2>
            </div>
            <Users size={19} className="panel-icon" />
          </div>
          <div className="search-box social-search">
            <Search size={14} />
            <input
              value={search}
              onChange={(event) => setSearch(event.target.value)}
              placeholder="Search friends"
            />
          </div>
          <div className="friend-list">
            {friends.map((friend) => (
              <button
                className={cn(
                  "friend-row",
                  !friend.online && "offline",
                  selected?.cid ===

                    (conversationFor(friend)?.cid ?? friend.pid?.split("/")[0] ?? friend.puuid) &&
                    "active",
                )}
                key={friend.puuid}
                onClick={() => openFriend(friend)}
              >
                <span className={friend.online ? "friend-presence online" : "friend-presence"} />
                <span className="friend-avatar">
                  {friend.online ? <Gamepad2 size={15} /> : friendName(friend).slice(0, 1)}
                </span>
                <span className="friend-copy">
                  <strong>{friendName(friend)}</strong>
                  <em>
                    {friend.online
                      ? presenceLabel(friend)
                      : `last seen ${relativeTime(friend.lastOnline)}`}
                  </em>
                </span>
                {friend.presence.partySize > 1 && (
                  <span className="friend-party">{friend.presence.partySize}</span>
                )}
              </button>
            ))}
            {data && friends.length === 0 && (
              <div className="owned-empty">
                <Users size={17} />
                <span>No friends matched.</span>
              </div>
            )}
          </div>

          <div className="social-request-block">
            <span className="panel-label">Friend requests</span>
            <div className="social-request-list">
              {(data?.requests ?? []).map((request) => (
                <div className="social-request" key={request.puuid}>
                  <span>
                    <strong>
                      {request.gameName || "Unknown"}#{request.tagLine}
                    </strong>
                    <em>{request.subscription === "pending_in" ? "incoming" : "outgoing"}</em>
                  </span>
                  <button
                    className="icon-button"
                    onClick={() => removeRequest(request.puuid)}
                    title="Remove request"
                    disabled={busy !== null}
                  >
                    <X size={13} />
                  </button>
                </div>
              ))}
              {(data?.requests.length ?? 0) === 0 && (
                <span className="social-muted">No pending requests.</span>
              )}
            </div>
            <div className="social-add-friend">
              <input
                value={requestName}
                onChange={(event) => setRequestName(event.target.value)}
                placeholder="Name"
              />
              <span>#</span>
              <input
                value={requestTag}
                onChange={(event) => setRequestTag(event.target.value)}
                placeholder="Tag"
              />
              <button
                className="button small"
                onClick={addFriend}
                disabled={busy !== null || !requestName.trim() || !requestTag.trim()}
              >
                {busy === "request" ? <Loader2 size={12} className="spin" /> : <UserPlus size={12} />}
                Add
              </button>
            </div>
          </div>
        </section>

        <section className="panel social-chat">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Riot chat</span>
              <h2>{selected?.name ?? "Select a friend"}</h2>
            </div>
            <MessageSquare size={19} className="panel-icon" />
          </div>

          {selected ? (
            <>
              <div className="chat-thread">
                {messages.length > 0 ? (
                  messages.map((message) => {
                    const mine = message.puuid === data?.puuid || message.name === "You";
                    return (
                      <div
                        className={cn("chat-message", mine && "mine")}
                        key={message.id || message.mid || `${message.time}-${message.body}`}
                      >
                        <div className="chat-bubble">
                          <span className="chat-author">
                            {message.puuid === data?.puuid
                              ? data?.account ?? "You"
                              : `${(message.game_name || message.name || "Unknown").trim()}${
                                  message.game_tag ? `#${message.game_tag}` : ""
                                }`}
                          </span>
                          <p>{message.body}</p>
                        </div>
                        <em>{messageTime(message.time)}</em>
                      </div>
                    );
                  })
                ) : (
                  <div className="owned-empty">
                    <MessageSquare size={17} />
                    <span>No messages in this conversation yet.</span>
                  </div>
                )}
              </div>
              <div className="chat-compose">
                <input
                  value={draft}
                  onChange={(event) => setDraft(event.target.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter" && !event.shiftKey) {
                      event.preventDefault();
                      void send();
                    }
                  }}
                  placeholder="Message through the local Riot client"
                />
                <button
                  className="button primary"
                  onClick={() => void send()}
                  disabled={busy !== null || !draft.trim()}
                >
                  {busy === "send" ? <Loader2 size={14} className="spin" /> : <Send size={14} />}
                  Send
                </button>
              </div>
            </>
          ) : (
            <div className="social-empty-chat">
              <MessageSquare size={32} />
              <strong>Pick a friend</strong>
              <span>Their conversation, presence and messages load here.</span>
            </div>
          )}
        </section>

        <section className="panel social-conversations">
          <div className="panel-heading">
            <div>
              <span className="panel-label">Conversations</span>
              <h2>Recent</h2>
            </div>
            <Clock3 size={19} className="panel-icon" />
          </div>
          <div className="conversation-list">
            {(data?.conversations ?? []).map((conversation) => {
              const others = conversation.participants.filter(
                (participant) => participant.puuid !== data?.puuid,
              );
              const label = others
                .map((participant) =>
                  `${participant.gameName}${participant.tagLine ? `#${participant.tagLine}` : ""}`,
                )
                .filter(Boolean)
                .join(", ");
              return (
                <button
                  className={cn("conversation-row", selected?.cid === conversation.cid && "active")}
                  key={conversation.cid}
                  onClick={() => openConversation(conversation)}
                >
                  <span className="friend-avatar">
                    {conversation.directMessages ? <MessageSquare size={14} /> : <Users size={14} />}
                  </span>
                  <span>
                    <strong>{label || conversation.cid}</strong>
                    <em>{conversation.type}</em>
                  </span>
                  {conversation.unread > 0 && <b>{conversation.unread}</b>}
                </button>
              );
            })}
            {(data?.conversations.length ?? 0) === 0 && (
              <span className="social-muted">No active conversations.</span>
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
