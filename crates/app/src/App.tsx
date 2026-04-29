import { useState } from "react";
import "./App.css";

// Interface ini mirip dengan struct RoomInfo di Rust
interface RoomInfo {
  room_code: string;
  status: string;
}

function App() {
  const [room, setRoom] = useState<RoomInfo | null>(null);
  const [loading, setLoading] = useState(false);

  const createRoom = async () => {
    setLoading(true);
    try {
      const response = await fetch("http://127.0.0.1:3000/room/create", {
        method: "POST",
      });
      const data: RoomInfo = await response.json();
      setRoom(data);
    } catch (error) {
      console.error("Gagal membuat room:", error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="container">
      <h1>Aplikasi Midman</h1>
      <p>Buat room transaksi aman sekarang.</p>

      <button onClick={createRoom} disabled={loading}>
        {loading ? "Membuat..." : "Buat Room Baru"}
      </button>

      {room && (
        <div className="room-card">
          <h2>Kode Room: {room.room_code}</h2>
          <p>Status: {room.status}</p>
        </div>
      )}
    </div>
  );
}

export default App;
