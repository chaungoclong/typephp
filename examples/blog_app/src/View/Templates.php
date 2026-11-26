<?php

declare(strict_types=1);
// Target: PHP 8.4

namespace TypePhp\BlogApp\View;

class Templates
{
    private function layout(string $title, string $body): string
    {
        return '<!DOCTYPE html>
<html lang="vi">
<head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>' . htmlspecialchars($title) . '</title>
    <script src="https://cdn.tailwindcss.com"></script>
</head>
<body class="bg-slate-50 text-slate-800 min-h-screen flex flex-col">
    <header class="bg-white border-b border-slate-200 sticky top-0 z-10">
        <div class="max-w-5xl mx-auto px-4 py-4 flex items-center justify-between">
            <a href="/" class="text-xl font-bold text-indigo-600 tracking-tight">TypePHP Blog</a>
            <nav class="flex items-center gap-4 text-sm font-medium">
                <a href="/" class="text-slate-600 hover:text-indigo-600 transition">Blog</a>
                <a href="/admin" class="px-3 py-1.5 rounded-lg bg-indigo-600 text-white hover:bg-indigo-700 transition">Admin</a>
            </nav>
        </div>
    </header>
    <main class="max-w-5xl w-full mx-auto px-4 py-8 flex-1">' . $body . '</main>
    <footer class="border-t border-slate-200 bg-white py-4 text-center text-xs text-slate-400">
        Powered by TypePHP PSR-7 Micro-Framework &amp; SQLite
    </footer>
</body>
</html>';
    }

    private function safeStr(mixed $val): string
    {
        if (is_string($val)) {
            return htmlspecialchars($val);
        }
        if (is_int($val)) {
            return (string) $val;
        }
        return '';
    }

    /**
     * @param array<int, mixed> $posts
     */
    public function renderBlogPage(array $posts): string
    {
        $cards = '';
        $count = 0;
        foreach ($posts as $row) {
            if (is_array($row)) {
                $count = $count + 1;
                $id = $this->safeStr($row['id'] ?? 0);
                $title = $this->safeStr($row['title'] ?? '');
                $slug = $this->safeStr($row['slug'] ?? '');
                $content = $this->safeStr($row['content'] ?? '');
                $created = $this->safeStr($row['created_at'] ?? '');
                $cards = $cards . '<article class="bg-white rounded-xl border border-slate-200 p-6 shadow-sm hover:shadow-md transition">
                    <div class="flex items-center justify-between text-xs text-slate-400 mb-2">
                        <span class="font-mono bg-slate-100 text-slate-600 px-2 py-0.5 rounded">/' . $slug . '</span>
                        <span>' . $created . '</span>
                    </div>
                    <h2 class="text-xl font-bold text-slate-900 mb-2">
                        <a href="/post?id=' . $id . '" class="hover:text-indigo-600 transition">' . $title . '</a>
                    </h2>
                    <p class="text-slate-600 text-sm leading-relaxed mb-4">' . $content . '</p>
                    <a href="/post?id=' . $id . '" class="text-sm font-semibold text-indigo-600 hover:text-indigo-800">Đọc tiếp &rarr;</a>
                </article>';
            }
        }
        if ($count === 0) {
            $cards = '<div class="bg-white rounded-xl border border-dashed border-slate-300 p-12 text-center">
                <p class="text-slate-500 mb-4">Chưa có bài viết nào được xuất bản.</p>
                <a href="/admin" class="inline-block px-4 py-2 rounded-lg bg-indigo-600 text-white text-sm font-medium hover:bg-indigo-700">Vào trang Admin tạo bài viết</a>
            </div>';
        }
        $body = '<div class="mb-8">
            <h1 class="text-3xl font-extrabold text-slate-900">Bài viết mới nhất</h1>
            <p class="text-slate-500 mt-1">Ứng dụng Blog viết bằng TypePHP + Tailwind CSS + SQLite</p>
        </div>
        <div class="grid gap-6">' . $cards . '</div>';
        return $this->layout('TypePHP Blog', $body);
    }

    public function renderPostDetailPage(mixed $row): string
    {
        if (!is_array($row)) {
            $notFound = '<div class="bg-white rounded-xl border border-slate-200 p-8 text-center">
                <h1 class="text-2xl font-bold text-slate-900 mb-2">Không tìm thấy bài viết</h1>
                <a href="/" class="text-indigo-600 hover:underline text-sm">&larr; Quay lại trang chủ</a>
            </div>';
            return $this->layout('404 - TypePHP Blog', $notFound);
        }
        $title = $this->safeStr($row['title'] ?? '');
        $slug = $this->safeStr($row['slug'] ?? '');
        $content = $this->safeStr($row['content'] ?? '');
        $created = $this->safeStr($row['created_at'] ?? '');
        $body = '<div class="mb-6">
            <a href="/" class="text-sm text-indigo-600 hover:underline">&larr; Quay lại danh sách</a>
        </div>
        <article class="bg-white rounded-xl border border-slate-200 p-8 shadow-sm">
            <div class="flex items-center gap-3 text-xs text-slate-400 mb-3">
                <span class="font-mono bg-slate-100 text-slate-600 px-2 py-0.5 rounded">/' . $slug . '</span>
                <span>' . $created . '</span>
            </div>
            <h1 class="text-3xl font-extrabold text-slate-900 mb-6">' . $title . '</h1>
            <div class="text-slate-700 leading-relaxed whitespace-pre-line">' . $content . '</div>
        </article>';
        return $this->layout($title . ' - TypePHP Blog', $body);
    }

    public function renderLoginPage(string $errorMsg = ''): string
    {
        $errHtml = '';
        if ($errorMsg !== '') {
            $errHtml = '<div class="mb-4 p-3 rounded-lg bg-red-50 border border-red-200 text-red-700 text-sm">' . htmlspecialchars($errorMsg) . '</div>';
        }
        $body = '<div class="max-w-md mx-auto bg-white rounded-xl border border-slate-200 p-8 shadow-sm mt-8">
            <h1 class="text-2xl font-extrabold text-slate-900 mb-2">Đăng nhập Admin</h1>
            <p class="text-xs text-slate-500 mb-6">Tài khoản mặc định: <span class="font-mono bg-slate-100 px-1.5 py-0.5 rounded">admin</span> / <span class="font-mono bg-slate-100 px-1.5 py-0.5 rounded">admin123</span></p>
            ' . $errHtml . '
            <form id="loginForm" class="space-y-4">
                <div>
                    <label class="block text-xs font-semibold text-slate-700 mb-1">Tên đăng nhập</label>
                    <input id="username" type="text" value="admin" required
                        class="w-full px-3 py-2 text-sm border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500" />
                </div>
                <div>
                    <label class="block text-xs font-semibold text-slate-700 mb-1">Mật khẩu</label>
                    <input id="password" type="password" value="admin123" required
                        class="w-full px-3 py-2 text-sm border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500" />
                </div>
                <button type="submit" class="w-full px-4 py-2 rounded-lg bg-indigo-600 text-white text-sm font-semibold hover:bg-indigo-700 transition">Đăng nhập</button>
            </form>
            <div id="loginErr" class="hidden mt-4 p-3 rounded-lg bg-red-50 border border-red-200 text-red-700 text-xs"></div>
        </div>
        <script>
        document.getElementById("loginForm").addEventListener("submit", async (e) => {
            e.preventDefault();
            const res = await fetch("/api/login", {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                    username: document.getElementById("username").value,
                    password: document.getElementById("password").value
                })
            });
            const json = await res.json();
            if (json.ok) {
                window.location.href = "/admin";
            } else {
                const box = document.getElementById("loginErr");
                box.textContent = json.error || "Đăng nhập thất bại";
                box.classList.remove("hidden");
            }
        });
        </script>';
        return $this->layout('Đăng nhập Admin - TypePHP Blog', $body);
    }

    public function renderAdminPage(): string
    {
        $body = '<div class="flex flex-col md:flex-row md:items-center md:justify-between gap-4 mb-8">
            <div>
                <h1 class="text-3xl font-extrabold text-slate-900">Blog Admin</h1>
                <p class="text-slate-500 text-sm mt-1">Quản lý bài viết qua PSR-7 API &amp; Validator</p>
            </div>
            <div class="flex items-center gap-3">
                <span class="text-xs px-2.5 py-1 rounded-full bg-emerald-50 text-emerald-700 font-medium">Đã đăng nhập: admin</span>
                <button id="logoutBtn" class="px-3 py-1.5 text-xs font-medium rounded-lg bg-slate-200 hover:bg-slate-300 text-slate-700 transition">Đăng xuất</button>
            </div>
        </div>

        <div class="grid md:grid-cols-3 gap-8">
            <section class="md:col-span-1 bg-white rounded-xl border border-slate-200 p-6 shadow-sm h-fit">
                <h2 id="formTitle" class="text-lg font-bold text-slate-900 mb-4">Tạo bài viết mới</h2>
                <div id="errorBox" class="hidden mb-4 p-3 rounded-lg bg-red-50 border border-red-200 text-red-700 text-xs space-y-1"></div>
                <form id="postForm" class="space-y-4">
                    <input type="hidden" id="postId" value="" />
                    <div>
                        <label class="block text-xs font-semibold text-slate-700 mb-1">Tiêu đề</label>
                        <input id="title" type="text" placeholder="Ví dụ: Giới thiệu TypePHP"
                            class="w-full px-3 py-2 text-sm border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500" />
                    </div>
                    <div>
                        <label class="block text-xs font-semibold text-slate-700 mb-1">Slug</label>
                        <input id="slug" type="text" placeholder="gioi-thieu-typephp"
                            class="w-full px-3 py-2 text-sm border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500" />
                    </div>
                    <div>
                        <label class="block text-xs font-semibold text-slate-700 mb-1">Nội dung</label>
                        <textarea id="content" rows="5" placeholder="Nội dung tối thiểu 10 ký tự..."
                            class="w-full px-3 py-2 text-sm border border-slate-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-indigo-500"></textarea>
                    </div>
                    <label class="flex items-center gap-2 text-sm text-slate-700 cursor-pointer">
                        <input id="published" type="checkbox" checked class="rounded text-indigo-600" />
                        <span>Xuất bản ngay</span>
                    </label>
                    <div class="flex gap-2">
                        <button type="submit" class="flex-1 px-4 py-2 rounded-lg bg-indigo-600 text-white text-sm font-semibold hover:bg-indigo-700 transition">Lưu bài viết</button>
                        <button type="button" id="cancelEdit" class="hidden px-3 py-2 rounded-lg bg-slate-100 text-slate-700 text-sm hover:bg-slate-200">Huỷ</button>
                    </div>
                </form>
            </section>

            <section class="md:col-span-2 bg-white rounded-xl border border-slate-200 p-6 shadow-sm">
                <div class="flex items-center justify-between mb-4">
                    <h2 class="text-lg font-bold text-slate-900">Danh sách bài viết</h2>
                    <button id="refreshBtn" class="text-xs px-3 py-1.5 rounded-lg bg-slate-100 hover:bg-slate-200 text-slate-700 font-medium">Làm mới</button>
                </div>
                <div class="overflow-x-auto">
                    <table class="w-full text-left text-sm">
                        <thead>
                            <tr class="border-b border-slate-200 text-xs text-slate-500 uppercase">
                                <th class="py-2 pr-3">ID</th>
                                <th class="py-2 pr-3">Tiêu đề</th>
                                <th class="py-2 pr-3">Trạng thái</th>
                                <th class="py-2 text-right">Thao tác</th>
                            </tr>
                        </thead>
                        <tbody id="postsBody" class="divide-y divide-slate-100"></tbody>
                    </table>
                </div>
            </section>
        </div>

        <script>
        const form = document.getElementById("postForm");
        const errorBox = document.getElementById("errorBox");
        const postsBody = document.getElementById("postsBody");
        const postIdEl = document.getElementById("postId");
        const titleEl = document.getElementById("title");
        const slugEl = document.getElementById("slug");
        const contentEl = document.getElementById("content");
        const publishedEl = document.getElementById("published");
        const formTitle = document.getElementById("formTitle");
        const cancelEdit = document.getElementById("cancelEdit");

        function headers() {
            return { "Content-Type": "application/json" };
        }

        document.getElementById("logoutBtn").addEventListener("click", async () => {
            await fetch("/api/logout", { method: "POST" });
            window.location.href = "/login";
        });

        function showErrors(errs) {
            errorBox.innerHTML = Object.values(errs).map(m => `<div>• ${m}</div>`).join("");
            errorBox.classList.remove("hidden");
        }

        function clearErrors() {
            errorBox.classList.add("hidden");
            errorBox.innerHTML = "";
        }

        function resetForm() {
            postIdEl.value = "";
            titleEl.value = "";
            slugEl.value = "";
            contentEl.value = "";
            publishedEl.checked = true;
            formTitle.textContent = "Tạo bài viết mới";
            cancelEdit.classList.add("hidden");
            clearErrors();
        }

        async function loadPosts() {
            const res = await fetch("/api/admin/posts", { headers: headers() });
            const json = await res.json();
            if (!json.ok) {
                postsBody.innerHTML = `<tr><td colspan="4" class="py-4 text-center text-red-500">${json.error || "Unauthorized"}</td></tr>`;
                return;
            }
            if (json.data.length === 0) {
                postsBody.innerHTML = `<tr><td colspan="4" class="py-6 text-center text-slate-400">Chưa có bài viết nào</td></tr>`;
                return;
            }
            postsBody.innerHTML = json.data.map(p => `
                <tr>
                    <td class="py-3 pr-3 font-mono text-xs text-slate-500">#${p.id}</td>
                    <td class="py-3 pr-3">
                        <div class="font-semibold text-slate-900">${p.title}</div>
                        <div class="text-xs text-slate-400 font-mono">/${p.slug}</div>
                    </td>
                    <td class="py-3 pr-3">
                        ${Number(p.published) === 1
                            ? `<span class="px-2 py-0.5 text-xs rounded-full bg-emerald-50 text-emerald-700 font-medium">Published</span>`
                            : `<span class="px-2 py-0.5 text-xs rounded-full bg-amber-50 text-amber-700 font-medium">Draft</span>`}
                    </td>
                    <td class="py-3 text-right space-x-2">
                        <button onclick="editPost(${p.id})" class="text-xs text-indigo-600 hover:underline font-medium">Sửa</button>
                        <button onclick="deletePost(${p.id})" class="text-xs text-red-600 hover:underline font-medium">Xoá</button>
                    </td>
                </tr>
            `).join("");
            window._posts = json.data;
        }

        window.editPost = function(id) {
            const p = (window._posts || []).find(x => Number(x.id) === Number(id));
            if (!p) return;
            postIdEl.value = p.id;
            titleEl.value = p.title;
            slugEl.value = p.slug;
            contentEl.value = p.content;
            publishedEl.checked = Number(p.published) === 1;
            formTitle.textContent = "Chỉnh sửa bài #" + p.id;
            cancelEdit.classList.remove("hidden");
            clearErrors();
        };

        window.deletePost = async function(id) {
            await fetch("/api/admin/posts/delete?id=" + id, { method: "DELETE", headers: headers() });
            await loadPosts();
        };

        cancelEdit.addEventListener("click", resetForm);
        document.getElementById("refreshBtn").addEventListener("click", loadPosts);

        form.addEventListener("submit", async (e) => {
            e.preventDefault();
            clearErrors();
            const payload = {
                title: titleEl.value,
                slug: slugEl.value,
                content: contentEl.value,
                published: publishedEl.checked
            };
            const id = postIdEl.value;
            const url = id ? "/api/admin/posts/update?id=" + id : "/api/admin/posts";
            const method = id ? "PUT" : "POST";
            const res = await fetch(url, { method, headers: headers(), body: JSON.stringify(payload) });
            const json = await res.json();
            if (!json.ok) {
                showErrors(json.errors || { error: json.error || "Failed" });
                return;
            }
            resetForm();
            await loadPosts();
        });

        loadPosts();
        </script>';
        return $this->layout('Admin - TypePHP Blog', $body);
    }
}
