### Database Page Specification

#### Overview:
This page will allow users to interact with a specific database, run queries, view results, and download results in CSV/JSON format. It will adjust its functionality and appearance based on user permissions, ensuring that only authorized users can access certain actions. The page will also include navigation back to the entity that owns the database.

Scaffold the work you do based on src/server/ui_endpoints/entity_details.rs, which by way of src/server/ui_endpoints/mod.rs, is included in src/server/server_runner.rs.


---

### 1. **Page Layout & Navigation**

#### **Breadcrumb Navigation:**
- The page will feature breadcrumb navigation to allow users to easily return to the entity's list of databases.
- The format will be `Entity Name / Database Name`.

#### **Metadata Display:**
At the top of the page, the following information will be displayed:
- **Database Name (Slug)**: The human-readable name of the database.
- **Database Type (db_type)**: 
  - 0 = SQLite
  - 1 = DuckDB
- **Entity Owner**: Display the `display_name` of the entity that owns this database.
  
#### **User Permissions:**
- User permissions will be passed as a `QueryMode` value (e.g., `None`, `ForkOnly`, `ReadOnly`, `ReadWrite`).
  - **No access** (`None`): Display an error message and link back to the entity's database list.
  - **Fork-only access** (`ForkOnly`): Query interface will be disabled with an explanation, inviting the user to fork the database.
  - **Read-only access** (`ReadOnly`): Query interface enabled, but modification queries will result in an error message.
  - **Read-write access** (`ReadWrite`): Full query execution allowed with no restrictions.

#### **Permissions Tab (for Managers/Owners):**
- Include a tab for editing the database's permissions, accessible only to database managers or owners.

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
- If a user without permission (e.g., `None` or `ForkOnly`) tries to run a query, show an error message explaining the permission level and provide a link to the entity's databases list or the option to fork the database.

---

### 5. **Data Handling**

#### **API/Backend Communication:**
- When the database page is rendered, the backend will pass the `QueryMode` (e.g., `None`, `ForkOnly`, `ReadOnly`, `ReadWrite`).
- The data passed should include:
  - **Database information**: Name, type (SQLite/DuckDB), and entity owner.
  - **User Permissions**: Based on the user’s `QueryMode`.

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
  
---

### 8. **Testing Plan**

#### **Unit Tests:**
- Test API responses for the correct `QueryMode` and proper database information.
- Test query execution to ensure correct handling of valid and invalid queries based on user permissions.

#### **Integration Tests:**
- Test the interaction between the frontend and backend by simulating different user roles and query submissions.
- Ensure correct behavior when results exceed 2000 rows and verify CSV/JSON downloads work as expected.

#### **UI/UX Tests:**
- Verify the visibility and behavior of the query input based on user permissions.
- Ensure the error display works as intended for different types of errors.
- Check the layout, breadcrumb navigation, and pagination for usability.

#### **Edge Cases:**
- Ensure that no more than 2000 rows are returned.
- Validate query errors, ensuring they’re displayed clearly and in the correct area.
- Test the behavior when no results are returned (e.g., empty query result).
  
