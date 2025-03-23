### Database Page Specification

#### Overview:
This page will allow users to interact with a specific database, run queries, view results, and download results in CSV/JSON format. It will adjust its functionality and appearance based on user permissions, ensuring that only authorized users can access certain actions. The page will also include navigation back to the entity that owns the database.

---

### 1. **Page Layout & Navigation**

#### **Breadcrumb Navigation:**
- The page will feature breadcrumb navigation to allow users to easily return to the entity's list of databases.
- The format will be `Entity Name / Database Name`.

#### **Metadata Display:**
At the top of the page, the following information will be displayed:
- **Database Name (Slug)**: The human-readable name of the database.
- **Entity Owner**: Display the entity slug that owns this database.
- **Database Type**: The type of database (SQLite or DuckDB).

These can be displayed as breadcrumbs, for example "marcua / test.sqlite (SQLite)" as the "{entity} / {database} ({database type})". The entity should link back to the entity's page at `/{entity_slug}/`.

Below that information, display three tabs: Query (the default and active tab for the page), Sharing, and Snapshots. Sharing and Snapshots should only be displayed when `can_manage_database` is true. In this version, we'll leave them as placeholders that tell you the `ayb client ...` command-line to run to modify sharing or list snapshots.

#### **User Permissions:**
- User permissions will be determined from the `database_details` endpoint response, which includes:
  - `highest_query_access_level`: Determines what query operations the user can perform
  - `can_manage_database`: Determines if the user can manage database permissions

- Based on the `highest_query_access_level` value:
  - **No access** (null/None): Query interface will be disabled with an explanation, inviting the user to request access or fork the database.
  - **Read-only access**: Query interface enabled, but modification queries will result in an error message.

---

### 2. **Query Interface**

#### **Interface Design:**
- A simple text box will be used for entering queries.
- No additional features like syntax highlighting or autocomplete are required in Version 1.

#### **Query Behavior:**
- The query interface will be displayed based on the user’s permissions:
  - **No access**: The query interface will be hidden, with an error displayed instead.
  - **Fork-only access**: The query interface will be visible but disabled, with a message explaining why.
  - **Read-only access**: The query interface will be enabled, but any modification queries will trigger an error.
  - **Read-write access**: Full query execution is allowed.

---

### 3. **Query Results**

#### **Display:**
- Query results will be displayed in a **table** by default, with a maximum of 2000 rows displayed.
- If results exceed 2000 rows, users will be notified and given the option to **download the full results** in **CSV** or **JSON** format.

#### **Pagination:**
- Query results will be paginated with 50 rows per page, with pagination controls provided below the table.

#### **Error Display:**
- If a query results in an error (e.g., syntax error, permission error), the error message will be displayed in place of the results table.
- Errors should be clearly communicated, with possible distinctions between types (e.g., query errors vs. permission errors).

---

### 4. **Error Handling**

#### **Query Errors:**
- **Query Syntax Errors**: Display the error message where the results table would be, indicating that the query was malformed.
- **Permission Errors**: If a user tries to modify the database without sufficient access, an error message should explain the permission issue.

#### **Permissions Errors:**
- If a user without permission tries to run a query, show an error message explaining the permission level and provide a link to the entity's databases list or the option to fork the database.

---

### 5. **Data Handling**

#### **API/Backend Communication:**
- When the database page is rendered, make a request to the database details endpoint to request the relevant context to render the template.
- The database page only renders HTML for the title/breadcrumbs/relevant tabs depending on permissions.
- Separate endpoints will actually execute the query (in src/server/ui_endpoints/query.rs, src/server/ui_endpoints/sharing.rs, and src/server/ui_endpoints/snapshots.rs)
- The database page will be in src/server/ui_endpoints/database.rs, which you can model off of src/server/ui_endpoints/entity_details.rs. The path to the page should be a http://server.domain/entity_slug/database_slug.
- The query endpoint will be in src/server/ui_endpoints/query.rs, and will accept a `format` parameter with values `html` or `json` or `csv`, and be served from http://server.domain/entity_slug/database_slug/query.
- The database page will render the query interface and results table, but will query (POST) the `query` endpoint and update the results using HTMX so that the entire page doesn't have to reload.


#### **Query Execution:**
- When the user submits a query, the frontend will send it to the backend for execution.
- The backend will handle query execution and return the results or errors.
- Pagination will be handled in the backend, returning a maximum of 2000 rows with 50 rows per page.

---

### 6. **Download Functionality**

- If the query results exceed 2000 rows, or if the user chooses to download the results, they will have the option to download them in either **CSV** or **JSON** format.
- Implement download buttons below the results table, appearing only when the user has run a query and has the option to download results.

---

### 7. **Design & Styling**

#### **UI Components:**
- Use **Franken-UI 2** components for all UI elements, including the query input box, result table, pagination controls, error message display, and download buttons.


